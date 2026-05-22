//! Tests for compact task-state and folder-note helpers.

#[cfg(test)]
mod tests {
    use crate::session_state::{load_folder_notes, record_task_finished};

    #[tokio::test]
    async fn folder_notes_include_child_task_summary_and_fallbacks() {
        let root = temp_root("dsx_folder_notes");
        let scoped = root.join("1234");
        let empty = root.join("empty");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&scoped).unwrap();
        std::fs::create_dir_all(&empty).unwrap();

        let pool = dsx_memory::open(&scoped.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let mut summary = dsx_memory::TaskSummary::new(&scoped.display().to_string());
        summary.last_changes = "patched workflow panel".into();
        summary.next_step = "run full gates".into();
        summary.architecture = "1234/: tron web app".into();
        dsx_memory::upsert_task_summary(&pool, &summary)
            .await
            .unwrap();

        let notes = load_folder_notes(&root).await;
        let scoped_note = notes.iter().find(|note| note.folder == "1234/").unwrap();
        let empty_note = notes.iter().find(|note| note.folder == "empty/").unwrap();

        assert_eq!(scoped_note.summary, "patched workflow panel");
        assert_eq!(scoped_note.next_step, "run full gates");
        assert_eq!(scoped_note.architecture, "1234/: tron web app");
        assert!(empty_note.summary.contains("project folder"));
        assert!(empty_note.architecture.contains("empty/"));

        pool.close().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn folder_notes_surface_scope_guard_summary() {
        let root = temp_root("dsx_folder_notes_scope_guard");
        let scoped = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&scoped).unwrap();

        let pool = dsx_memory::open(&scoped.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let mut summary = dsx_memory::TaskSummary::new(&scoped.display().to_string());
        summary.scope_violations = 2;
        summary.last_scope_violation = "grep: denied by active scope".into();
        dsx_memory::upsert_task_summary(&pool, &summary)
            .await
            .unwrap();

        let notes = load_folder_notes(&root).await;
        let scoped_note = notes.iter().find(|note| note.folder == "1234/").unwrap();

        assert!(scoped_note.summary.contains("Scope guard blocked 2"));
        assert!(scoped_note.summary.contains("grep"));

        pool.close().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn finished_task_persists_capsule_constraints_and_architecture() {
        let root = temp_root("dsx_task_finished_capsule");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let brief = dsx_tui::TaskBriefPanel {
            goal: "build".into(),
            constraints: "keep files <= 300 lines".into(),
            architecture: "- src/: app code".into(),
            active_scope: root.display().to_string(),
            ..Default::default()
        };

        record_task_finished(&root, &brief, &[], 0, "")
            .await
            .unwrap();
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let summary = dsx_memory::load_task_summary(&pool, &root.display().to_string())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(summary.constraints, "keep files <= 300 lines");
        assert_eq!(summary.architecture, "- src/: app code");

        pool.close().await;
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
