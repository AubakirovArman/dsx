//! Tests for compact task-state and folder-note helpers.

#[cfg(test)]
mod tests {
    use crate::session_state::load_folder_notes;

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
        dsx_memory::upsert_task_summary(&pool, &summary)
            .await
            .unwrap();

        let notes = load_folder_notes(&root).await;
        let scoped_note = notes.iter().find(|note| note.folder == "1234/").unwrap();
        let empty_note = notes.iter().find(|note| note.folder == "empty/").unwrap();

        assert_eq!(scoped_note.summary, "patched workflow panel");
        assert_eq!(scoped_note.next_step, "run full gates");
        assert!(empty_note.summary.contains("project folder"));

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
