//! Tests for TUI launch and active-scope indexing behavior.

#[cfg(test)]
mod tests {
    use crate::tui_state::{configure_initial_app, index_active_scope};
    use std::sync::{Arc, Mutex};

    #[test]
    fn initial_file_tree_is_shallow_launch_listing() {
        let root = temp_root("dsx_tui_shallow_tree");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234/src")).unwrap();
        std::fs::write(root.join("top.txt"), "top").unwrap();
        std::fs::write(root.join("1234/src/main.rs"), "fn main() {}").unwrap();

        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        configure_initial_app(
            &app,
            &root,
            dsx_core::types::PermissionMode::Ask,
            "https://api.deepseek.com".into(),
            String::new(),
            None,
        );

        let file_tree = app.lock().unwrap().file_tree.clone();
        assert!(file_tree.contains(&"top.txt".into()));
        assert!(file_tree.contains(&"1234/".into()));
        assert!(!file_tree.iter().any(|path| path.contains("main.rs")));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn active_scope_indexing_ignores_sibling_projects() {
        let root = temp_root("dsx_tui_active_index");
        let active = root.join("1234");
        let sibling = root.join("other");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(active.join("src")).unwrap();
        std::fs::create_dir_all(sibling.join("src")).unwrap();
        std::fs::write(active.join("src/lib.rs"), "pub fn active_only() {}\n").unwrap();
        std::fs::write(sibling.join("src/lib.rs"), "pub fn sibling_only() {}\n").unwrap();

        index_active_scope(&active).await.unwrap();
        let pool = dsx_memory::open(&active.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let active_hits = dsx_index::search_symbols(&active, &pool, "active_only", 10)
            .await
            .unwrap();
        let sibling_hits = dsx_index::search_symbols(&active, &pool, "sibling_only", 10)
            .await
            .unwrap();

        assert_eq!(active_hits.len(), 1);
        assert!(sibling_hits.is_empty());

        let _ = pool.close().await;
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
