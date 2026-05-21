//! Tests for TUI task lifecycle behavior.

#[cfg(test)]
mod tests {
    use crate::tui_task::{agent_workspace, prepare_task, stop_agent_task};
    use std::sync::{Arc, Mutex};

    #[test]
    fn prepare_task_blocks_parallel_run_and_keeps_input() {
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        {
            let mut app = app.lock().unwrap();
            app.input = "second task".into();
            app.agent_task = dsx_tui::AgentTask::Running("first task".into());
        }

        let prepared = prepare_task(&app, std::path::Path::new("/tmp"), "key");

        assert!(prepared.is_none());
        let app = app.lock().unwrap();
        assert_eq!(app.input, "second task");
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("already running"))
        );
    }

    #[test]
    fn prepare_task_allows_idle_task() {
        let root = temp_root("dsx_prepare_task_idle");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        app.lock().unwrap().input = "do work".into();

        let prepared = prepare_task(&app, &root, "key").unwrap();

        assert_eq!(prepared.task, "do work");
        assert_eq!(prepared.api_key, "key");
        assert_eq!(prepared.run_id, 1);
        assert_eq!(app.lock().unwrap().active_run_id, Some(1));
        assert!(matches!(
            app.lock().unwrap().agent_task,
            dsx_tui::AgentTask::Running(_)
        ));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_task_updates_scope_lock_for_subfolder() {
        let root = temp_root("dsx_prepare_scope_lock");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        app.lock().unwrap().input = "почини 1234".into();

        let prepared = prepare_task(&app, &root, "key").unwrap();

        assert_eq!(prepared.active_root, target.canonicalize().unwrap());
        assert_eq!(agent_workspace(&prepared), target.canonicalize().unwrap());
        let app = app.lock().unwrap();
        assert_eq!(app.scope_lock.launch_scope, root.display().to_string());
        assert_eq!(
            app.scope_lock.active_scope,
            target.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(app.scope_lock.status, "Narrowed");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_task_blocks_wide_container_scope_and_keeps_input() {
        let root = temp_root("dsx_prepare_wide_container");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        app.lock().unwrap().input = "доработай проект".into();

        let prepared = prepare_task(&app, &root, "key");

        assert!(prepared.is_none());
        let app = app.lock().unwrap();
        assert_eq!(app.input, "доработай проект");
        assert_eq!(app.active_run_id, None);
        assert_eq!(app.task_brief.done, "Task blocked before model call.");
        assert!(app.task_brief.next_step.contains("1234/"));
        assert_eq!(app.scope_lock.status, "Blocked");
        assert!(app.scope_lock.warning.contains("No model call"));
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Wide container workspace blocked"))
        );
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Agent preflight")
                    && msg.content.contains("Decision: BLOCKED"))
        );
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Policy source: container_guard"))
        );
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Suggested child scopes: 1234/"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_task_allows_wide_container_when_policy_enabled() {
        let root = temp_root("dsx_prepare_allow_wide");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        {
            let mut app = app.lock().unwrap();
            app.input = "доработай проект".into();
            app.allow_wide_scope = true;
        }

        let prepared = prepare_task(&app, &root, "key").unwrap();

        assert_eq!(prepared.active_root, root.canonicalize().unwrap());
        let app = app.lock().unwrap();
        assert_eq!(app.active_run_id, Some(1));
        assert_eq!(app.scope_lock.status, "Wide");
        let _ = std::fs::remove_dir_all(&root);
    }

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

        assert!(stop_agent_task(&app, &tokio::runtime::Handle::current()));

        assert!(!rx.await.unwrap());
        assert!(task.await.unwrap_err().is_cancelled());
        let app = app.lock().unwrap();
        assert!(app.agent_abort.is_none());
        assert!(app.pending_approval.is_none());
        assert_eq!(app.active_run_id, None);
        assert!(matches!(app.agent_task, dsx_tui::AgentTask::Error(_)));
    }

    #[tokio::test]
    async fn stop_agent_task_records_cancelled_run() {
        let root = temp_root("dsx_cancelled_run_ledger");
        std::fs::create_dir_all(&root).unwrap();
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let ledger_id = dsx_memory::start_agent_run(
            &pool,
            Some("sid"),
            &root.display().to_string(),
            "long task",
        )
        .await
        .unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        let task = tokio::spawn(async { std::future::pending::<()>().await });
        {
            let mut app = app.lock().unwrap();
            app.agent_abort = Some(task.abort_handle());
            app.active_run_id = Some(9);
            app.active_ledger_id = Some(ledger_id.clone());
            app.agent_task = dsx_tui::AgentTask::Running("work".into());
            app.task_brief.active_scope = root.display().to_string();
        }

        assert!(stop_agent_task(&app, &tokio::runtime::Handle::current()));
        let run = wait_for_run_status(&pool, &ledger_id, "cancelled").await;
        assert_eq!(run.status, "cancelled");
        assert!(run.cancelled);
        assert!(run.finished_at.is_some());

        let _ = pool.close().await;
        let _ = std::fs::remove_dir_all(root);
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

    async fn wait_for_run_status(
        pool: &sqlx::SqlitePool,
        ledger_id: &str,
        expected: &str,
    ) -> dsx_memory::AgentRunRecord {
        for _ in 0..20 {
            let run = dsx_memory::load_agent_run(pool, ledger_id)
                .await
                .unwrap()
                .unwrap();
            if run.status == expected {
                return run;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        dsx_memory::load_agent_run(pool, ledger_id)
            .await
            .unwrap()
            .unwrap()
    }
}
