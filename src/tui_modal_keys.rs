//! Keyboard handlers for modal TUI views.

use crate::tui_keys::KeyOutcome;
use crate::tui_state::SharedApp;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) fn handle_tools_key(key: KeyEvent, app: &SharedApp) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => close(app, |app| app.show_tools = false),
        KeyCode::Char('l') if ctrl(key) => close(app, |app| app.show_tools = false),
        KeyCode::Char('c') if ctrl(key) => KeyOutcome::Quit,
        _ => KeyOutcome::Continue,
    }
}

pub(crate) fn handle_mission_key(key: KeyEvent, app: &SharedApp) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => close(app, |app| app.show_mission = false),
        KeyCode::Char('m') if ctrl(key) => close(app, |app| app.show_mission = false),
        KeyCode::Char('c') if ctrl(key) => KeyOutcome::Quit,
        _ => KeyOutcome::Continue,
    }
}

pub(crate) fn handle_context_key(key: KeyEvent, app: &SharedApp) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => close(app, |app| app.show_context = false),
        KeyCode::Char('b') if ctrl(key) => close(app, |app| app.show_context = false),
        KeyCode::Enter => {
            app.lock().unwrap().draft_focused_scope_task();
            KeyOutcome::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.lock().unwrap().select_next_folder_note();
            KeyOutcome::Continue
        }
        KeyCode::Up | KeyCode::Char('k') if !ctrl(key) => {
            app.lock().unwrap().select_previous_folder_note();
            KeyOutcome::Continue
        }
        KeyCode::Char('c') if ctrl(key) => KeyOutcome::Quit,
        _ => KeyOutcome::Continue,
    }
}

fn close(app: &SharedApp, f: impl FnOnce(&mut dsx_tui::App)) -> KeyOutcome {
    f(&mut app.lock().unwrap());
    KeyOutcome::Continue
}

fn ctrl(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
}
