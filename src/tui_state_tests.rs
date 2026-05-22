//! Tests for TUI launch and active-scope indexing behavior.

#[cfg(test)]
mod tests {
    use crate::tui_state::{configure_initial_app, index_active_scope, load_startup_audit};
    use std::path::Path;
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
            false,
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

    #[tokio::test]
    async fn startup_audit_surfaces_scope_escape_warning() {
        let root = temp_root("dsx_tui_startup_audit");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();
        seed_scoped_run(&root, &child).await;

        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        configure_initial_app(
            &app,
            &root,
            dsx_core::types::PermissionMode::Ask,
            "https://api.deepseek.com".into(),
            String::new(),
            false,
            None,
        );
        load_startup_audit(&app, &root).await;

        let app = app.lock().unwrap();
        assert!(
            app.messages
                .iter()
                .any(|message| message.content.contains("Workspace audit:"))
        );
        assert!(app.scope_lock.warning.contains("scope escape"));
        assert_eq!(app.run_ledger.total, 1);
        assert_eq!(app.run_ledger.scope_violations, 2);
        assert_eq!(app.run_ledger.recent[0].scope, "1234");

        let _ = std::fs::remove_dir_all(root);
    }

    async fn seed_scoped_run(root: &Path, child: &Path) {
        let pool = dsx_memory::open(&child.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let id = dsx_memory::start_scoped_agent_run(
            &pool,
            &dsx_memory::AgentRunStart {
                session_id: None,
                project_root: &child.display().to_string(),
                task: "use only child scope",
                launch_scope: &root.display().to_string(),
                active_scope: &child.display().to_string(),
                scope_status: "Narrowed",
                scope_reason: "User selected a child project.",
            },
        )
        .await
        .unwrap();
        dsx_memory::finish_agent_run(
            &pool,
            &id,
            &dsx_memory::AgentRunUpdate {
                status: "completed".into(),
                scope_violations: 2,
                last_scope_violation: "read_file denied outside active scope".into(),
                ..Default::default()
            },
        )
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
