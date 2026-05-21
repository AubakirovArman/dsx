//! TUI run-ledger start helpers.

use crate::tui_state::SharedApp;

pub(crate) async fn start_run_ledger(
    app: &SharedApp,
    session_id: &Option<String>,
    task: &crate::tui_task::PreparedTask,
) -> Option<String> {
    let contract = {
        let app = app.lock().unwrap();
        crate::run_ledger::RunScopeContract::from_app(&app)
    };
    let result = crate::run_ledger::record_started(
        &task.active_root,
        session_id.as_deref(),
        &task.task,
        contract,
    )
    .await;

    match result {
        Ok(id) => {
            app.lock().unwrap().active_ledger_id = Some(id.clone());
            Some(id)
        }
        Err(e) => {
            app.lock()
                .unwrap()
                .add_message("error", &format!("Run ledger start failed: {e}"));
            None
        }
    }
}
