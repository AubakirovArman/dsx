//! Tests for live TUI run-ledger refresh after task lifecycle events.

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn finish_task_refreshes_run_ledger_panel() {
        let root = temp_root("dsx_finish_refresh_run_health");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let ledger_id = dsx_memory::start_agent_run(
            &pool,
            Some("sid"),
            &root.display().to_string(),
            "finish task",
        )
        .await
        .unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        {
            let mut state = app.lock().unwrap();
            state.active_run_id = Some(1);
            state.active_ledger_id = Some(ledger_id);
            state.agent_task = dsx_tui::AgentTask::Running("finish task".into());
            state.task_brief.active_scope = root.display().to_string();
            state.scope_lock.launch_scope = root.display().to_string();
            state.tokens = 21;
        }

        crate::tui_task::finish_task(
            app.clone(),
            None,
            None,
            root.clone(),
            1,
            tokio::runtime::Handle::current(),
        );
        let panel = wait_for_panel_total(&app, 1).await;

        assert_eq!(panel.recent[0].status, "completed");
        assert_eq!(panel.total_tokens, 21);

        let _ = pool.close().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn stop_agent_task_refreshes_cancelled_run_health() {
        let root = temp_root("dsx_cancel_refresh_run_health");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let ledger_id =
            dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), "stop task")
                .await
                .unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        let task = tokio::spawn(async { std::future::pending::<()>().await });
        {
            let mut state = app.lock().unwrap();
            state.agent_abort = Some(task.abort_handle());
            state.active_run_id = Some(2);
            state.active_ledger_id = Some(ledger_id);
            state.agent_task = dsx_tui::AgentTask::Running("stop task".into());
            state.task_brief.active_scope = root.display().to_string();
            state.scope_lock.launch_scope = root.display().to_string();
        }

        assert!(crate::tui_task::stop_agent_task(
            &app,
            &tokio::runtime::Handle::current()
        ));
        let panel = wait_for_panel_total(&app, 1).await;

        assert_eq!(panel.cancelled, 1);
        assert_eq!(panel.recent[0].status, "cancelled");

        let _ = task.await;
        let _ = pool.close().await;
        let _ = std::fs::remove_dir_all(root);
    }

    async fn wait_for_panel_total(
        app: &Arc<Mutex<dsx_tui::App>>,
        expected: usize,
    ) -> dsx_tui::RunLedgerPanel {
        for _ in 0..30 {
            let panel = app.lock().unwrap().run_ledger.clone();
            if panel.total == expected {
                return panel;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        app.lock().unwrap().run_ledger.clone()
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
