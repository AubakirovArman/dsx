//! TUI keyboard event handling.

use crate::tui_state::SharedApp;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;
use tokio::runtime::Handle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
    Continue,
    Quit,
}

pub async fn handle_key(
    key: KeyEvent,
    app: &SharedApp,
    project_root: &Path,
    api_key: &str,
    session_id: &Option<String>,
    pool: &Option<sqlx::SqlitePool>,
    rt: &Handle,
) -> anyhow::Result<KeyOutcome> {
    if let Some(outcome) = handle_modal_key(key, app) {
        return Ok(outcome);
    }

    match key.code {
        KeyCode::Char('c') if ctrl(key) => Ok(KeyOutcome::Quit),
        KeyCode::Enter => {
            crate::tui_task::start_agent_task(app, project_root, api_key, session_id, pool, rt)
                .await?;
            Ok(KeyOutcome::Continue)
        }
        KeyCode::Char('t') if ctrl(key) => toggle(app, |a| a.show_file_tree = !a.show_file_tree),
        KeyCode::Char('s') if ctrl(key) => toggle(app, |a| a.show_settings = !a.show_settings),
        KeyCode::Char('d') if ctrl(key) => toggle_diff(app, project_root),
        KeyCode::Char('u') if ctrl(key) => rollback(app, project_root),
        KeyCode::Up => scroll(app, 1),
        KeyCode::Down => scroll(app, -1),
        KeyCode::PageUp => scroll(app, 10),
        KeyCode::PageDown => scroll(app, -10),
        KeyCode::Char(ch) => edit_input(app, |input| input.push(ch)),
        KeyCode::Backspace => edit_input(app, |input| {
            input.pop();
        }),
        KeyCode::Esc => edit_input(app, |input| input.clear()),
        _ => Ok(KeyOutcome::Continue),
    }
}

fn handle_modal_key(key: KeyEvent, app: &SharedApp) -> Option<KeyOutcome> {
    if app.lock().unwrap().pending_approval.is_some() {
        return Some(handle_approval_key(key, app));
    }
    if app.lock().unwrap().show_diff {
        return Some(handle_diff_key(key, app));
    }
    if app.lock().unwrap().show_settings {
        return Some(crate::tui_settings_keys::handle_settings_key(app, key));
    }
    None
}

fn handle_approval_key(key: KeyEvent, app: &SharedApp) -> KeyOutcome {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            answer_approval(app, true);
            KeyOutcome::Continue
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            answer_approval(app, false);
            KeyOutcome::Continue
        }
        KeyCode::Char('c') if ctrl(key) => KeyOutcome::Quit,
        _ => KeyOutcome::Continue,
    }
}

fn handle_diff_key(key: KeyEvent, app: &SharedApp) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => {
            app.lock().unwrap().show_diff = false;
            KeyOutcome::Continue
        }
        KeyCode::Char('d') if ctrl(key) => {
            app.lock().unwrap().show_diff = false;
            KeyOutcome::Continue
        }
        KeyCode::Char('c') if ctrl(key) => KeyOutcome::Quit,
        _ => KeyOutcome::Continue,
    }
}

fn answer_approval(app: &SharedApp, approved: bool) {
    let mut app = app.lock().unwrap();
    if let Some(approval) = app.pending_approval.take() {
        let _ = approval.tx.send(approved);
        let msg = if approved {
            "🔐 Authorization: APPROVED (tool executing...)"
        } else {
            "🔒 Authorization: DENIED (tool blocked)"
        };
        app.add_message("system", msg);
    }
}

fn toggle(app: &SharedApp, f: impl FnOnce(&mut dsx_tui::App)) -> anyhow::Result<KeyOutcome> {
    f(&mut app.lock().unwrap());
    Ok(KeyOutcome::Continue)
}

fn toggle_diff(app: &SharedApp, project_root: &Path) -> anyhow::Result<KeyOutcome> {
    let mut app = app.lock().unwrap();
    if !app.show_diff {
        app.current_diff = dsx_git::diff(project_root)
            .unwrap_or_else(|_| "Error: Failed to fetch git diff.".into());
    }
    app.show_diff = !app.show_diff;
    Ok(KeyOutcome::Continue)
}

fn rollback(app: &SharedApp, project_root: &Path) -> anyhow::Result<KeyOutcome> {
    let mut app = app.lock().unwrap();
    match dsx_git::rollback(project_root) {
        Ok(msg) => {
            app.add_message("system", &format!("⏪ Workspace Reverted: {msg}"));
            if let Ok(files) = dsx_index::scan_project(project_root) {
                app.file_tree = files.into_iter().take(50).collect();
            }
        }
        Err(e) => app.add_message("error", &format!("🔒 Undo Failed: {e}")),
    }
    Ok(KeyOutcome::Continue)
}

fn scroll(app: &SharedApp, delta: i16) -> anyhow::Result<KeyOutcome> {
    let mut app = app.lock().unwrap();
    if app.input.is_empty() {
        if delta >= 0 {
            app.scroll_offset = app.scroll_offset.saturating_add(delta as u16);
        } else {
            app.scroll_offset = app.scroll_offset.saturating_sub((-delta) as u16);
        }
    }
    Ok(KeyOutcome::Continue)
}

fn edit_input(app: &SharedApp, f: impl FnOnce(&mut String)) -> anyhow::Result<KeyOutcome> {
    let mut app = app.lock().unwrap();
    f(&mut app.input);
    app.scroll_offset = 0;
    Ok(KeyOutcome::Continue)
}

fn ctrl(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
}
