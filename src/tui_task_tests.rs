//! Tests for TUI task lifecycle behavior.

#[cfg(test)]
mod tests {
    use crate::tui_task::stop_agent_task;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn stop_agent_task_aborts_handle_and_clears_state() {
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        let task = tokio::spawn(async { std::future::pending::<()>().await });
        let abort = task.abort_handle();
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut app = app.lock().unwrap();
            app.agent_abort = Some(abort);
            app.active_run_id = Some(7);
            app.agent_task = dsx_tui::AgentTask::Running("work".into());
            app.pending_approval = Some(dsx_tui::PendingApproval {
                tool_name: "run_command".into(),
                arguments: "{}".into(),
                tx,
            });
        }

        assert!(stop_agent_task(&app));

        assert!(!rx.await.unwrap());
        assert!(task.await.unwrap_err().is_cancelled());
        let app = app.lock().unwrap();
        assert!(app.agent_abort.is_none());
        assert!(app.pending_approval.is_none());
        assert_eq!(app.active_run_id, None);
        assert!(matches!(app.agent_task, dsx_tui::AgentTask::Error(_)));
    }

    #[tokio::test]
    async fn finish_task_preserves_error_state() {
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        let root = temp_root("dsx_finish_error");
        std::fs::create_dir_all(&root).unwrap();
        {
            let mut app = app.lock().unwrap();
            app.active_run_id = Some(3);
            app.agent_task = dsx_tui::AgentTask::Error("api failed".into());
        }

        crate::tui_task::finish_task(
            app.clone(),
            None,
            None,
            root.clone(),
            3,
            tokio::runtime::Handle::current(),
        );

        {
            let app = app.lock().unwrap();
            assert!(matches!(app.agent_task, dsx_tui::AgentTask::Error(_)));
            assert_eq!(app.active_run_id, None);
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
