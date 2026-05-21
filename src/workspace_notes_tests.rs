//! Tests for workspace task-summary note listing.

#[cfg(test)]
mod tests {
    use crate::workspace_notes::{collect_workspace_notes, notes_json_value};

    #[tokio::test]
    async fn notes_collect_saved_root_and_child_fallbacks() {
        let root = temp_root("dsx_workspace_notes");
        let child = root.join("1234");
        let hidden = root.join(".hidden");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();
        std::fs::create_dir_all(&hidden).unwrap();

        seed_summary(&root, "root goal", "root plan").await;
        let notes = collect_workspace_notes(&root, 10, true).await.unwrap();

        assert!(notes.iter().any(|note| note.label == "."));
        assert!(notes.iter().any(|note| note.label == "1234"));
        assert!(!notes.iter().any(|note| note.label == ".hidden"));
        let root_note = notes.iter().find(|note| note.label == ".").unwrap();
        let child_note = notes.iter().find(|note| note.label == "1234").unwrap();
        assert_eq!(root_note.goal, "root goal");
        assert_eq!(root_note.plan, "root plan");
        assert!(!child_note.saved);

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn notes_default_stays_on_launch_scope() {
        let root = temp_root("dsx_workspace_notes_default");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();

        let notes = collect_workspace_notes(&root, 10, false).await.unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].label, ".");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn notes_json_contains_context_fields() {
        let root = temp_root("dsx_workspace_notes_json");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        seed_summary(&root, "json goal", "json plan").await;

        let notes = collect_workspace_notes(&root, 1, false).await.unwrap();
        let value = notes_json_value(&notes);
        let first = value.as_array().unwrap().first().unwrap();

        assert_eq!(first["scope"], ".");
        assert_eq!(first["saved"], true);
        assert_eq!(first["goal"], "json goal");
        assert_eq!(first["plan"], "json plan");
        assert!(first.get("architecture").is_some());

        let _ = std::fs::remove_dir_all(root);
    }

    async fn seed_summary(root: &std::path::Path, goal: &str, plan: &str) {
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let mut summary = dsx_memory::TaskSummary::new(&root.display().to_string());
        summary.goal = goal.into();
        summary.done = "done".into();
        summary.plan = plan.into();
        summary.last_changes = "last".into();
        summary.next_step = "next".into();
        dsx_memory::upsert_task_summary(&pool, &summary)
            .await
            .unwrap();
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
