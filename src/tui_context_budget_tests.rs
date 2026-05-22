//! Tests for TUI context budget preflight.

#[cfg(test)]
mod tests {
    use crate::tui_context_budget::{block_with_error, preflight_context_budget};
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn context_budget_preflight_updates_visible_task_state() {
        let root = temp_root("dsx_tui_context_budget");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(child.join("src")).unwrap();
        std::fs::write(child.join("src").join("main.rs"), "fn main() {}\n").unwrap();
        let app = shared_app();
        app.lock().unwrap().begin_task_scoped(
            "доработай 1234",
            &root.display().to_string(),
            &child.display().to_string(),
            true,
        );

        let ok = preflight_context_budget(&app, "доработай 1234").await;
        let expected_scope = child.canonicalize().unwrap().display().to_string();
        let app = app.lock().unwrap();

        assert!(ok);
        assert!(app.budget_status.contains("capsule request"));
        assert_eq!(app.task_brief.goal, "доработай 1234");
        assert!(app.task_brief.done.contains("Context capsule"));
        assert!(app.task_brief.plan.contains("Inspect only"));
        assert!(app.task_brief.last_changes.contains("capsule request"));
        assert!(app.task_brief.next_step.contains("Start scoped inspection"));
        assert_eq!(app.task_brief.active_scope, expected_scope);
        assert!(app.task_brief.constraints.contains("300 lines"));
        assert!(app.task_brief.architecture.contains("src/"));
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Context budget preflight"))
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn context_budget_block_restores_task_without_model_call() {
        let app = shared_app();
        {
            let mut app = app.lock().unwrap();
            app.active_run_id = Some(7);
            app.task_brief.active_scope = "/tmp/sites/1234".into();
        }

        block_with_error(&app, "retry 1234", "over budget");
        let app = app.lock().unwrap();

        assert_eq!(app.active_run_id, None);
        assert_eq!(app.input, "retry 1234");
        assert!(app.task_brief.done.contains("blocked"));
        assert!(app.scope_lock.warning.contains("No model call"));
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("over budget"))
        );
    }

    fn shared_app() -> crate::tui_state::SharedApp {
        Arc::new(Mutex::new(dsx_tui::App::new()))
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
