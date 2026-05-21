//! Settings-screen keyboard handling.

use crate::tui_keys::KeyOutcome;
use crate::tui_state::SharedApp;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_settings_key(app: &SharedApp, key: KeyEvent) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => {
            app.lock().unwrap().show_settings = false;
            KeyOutcome::Continue
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.lock().unwrap().show_settings = false;
            KeyOutcome::Continue
        }
        KeyCode::Up => {
            let mut app = app.lock().unwrap();
            app.settings_cursor = app.settings_cursor.saturating_sub(1);
            KeyOutcome::Continue
        }
        KeyCode::Down => {
            let mut app = app.lock().unwrap();
            app.settings_cursor = (app.settings_cursor + 1).min(6);
            KeyOutcome::Continue
        }
        KeyCode::Left | KeyCode::Right => {
            toggle_selected_setting(app, matches!(key.code, KeyCode::Left));
            KeyOutcome::Continue
        }
        KeyCode::Enter => {
            clear_history_if_selected(app);
            KeyOutcome::Continue
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyOutcome::Quit,
        _ => KeyOutcome::Continue,
    }
}

fn toggle_selected_setting(app: &SharedApp, left: bool) {
    let mut app = app.lock().unwrap();
    match app.settings_cursor {
        0 => {
            let current = dsx_core::types::PermissionMode::parse(&app.mode)
                .unwrap_or(dsx_core::types::PermissionMode::Ask);
            let all = dsx_core::types::PermissionMode::all();
            let idx = all.iter().position(|x| *x == current).unwrap_or(2);
            let offset = if left { all.len() - 1 } else { 1 };
            app.mode = all[(idx + offset) % all.len()].as_str().to_string();
        }
        1 => {
            app.model = if app.model == "v4-pro" {
                "v4-flash"
            } else {
                "v4-pro"
            }
            .into();
        }
        2 => app.show_file_tree = !app.show_file_tree,
        3 => toggle_language(&mut app, left),
        4 => toggle_api_base(&mut app, left),
        5 => {
            let lang = app.lang;
            app.add_message("system", api_key_notice(lang));
        }
        _ => {}
    }
}

fn toggle_language(app: &mut dsx_tui::App, left: bool) {
    let all = dsx_tui::Language::all();
    let idx = all.iter().position(|x| *x == app.lang).unwrap_or(0);
    let offset = if left { all.len() - 1 } else { 1 };
    let next = all[(idx + offset) % all.len()];
    app.lang = next;
    app.add_message("system", language_notice(next));
}

fn toggle_api_base(app: &mut dsx_tui::App, left: bool) {
    let presets = [
        "https://api.deepseek.com",
        "http://localhost:11434/v1",
        "http://localhost:8000/v1",
        "https://api.openai.com/v1",
    ];
    let idx = presets.iter().position(|x| *x == app.api_base).unwrap_or(0);
    let offset = if left { presets.len() - 1 } else { 1 };
    let next = presets[(idx + offset) % presets.len()];
    app.api_base = next.to_string();
    app.add_message("system", &format!("API Endpoint base changed to: {next}"));
}

fn clear_history_if_selected(app: &SharedApp) {
    let mut app = app.lock().unwrap();
    if app.settings_cursor != 6 {
        return;
    }
    app.messages.clear();
    let lang = app.lang;
    app.add_message("system", clear_notice(lang));
}

fn language_notice(lang: dsx_tui::Language) -> &'static str {
    match lang {
        dsx_tui::Language::Russian => "Язык интерфейса изменен на Русский.",
        dsx_tui::Language::Kazakh => "Интерфейс тілі Қазақша болып өзгертілді.",
        dsx_tui::Language::Chinese => "界面显示语言已成功切换为 中文。",
        dsx_tui::Language::English => "Interface language successfully changed to English.",
    }
}

fn api_key_notice(lang: dsx_tui::Language) -> &'static str {
    match lang {
        dsx_tui::Language::Russian => "🔑 Системный лог: Ключ API загружен из окружения.",
        dsx_tui::Language::Kazakh => "🔑 Жүйелік журнал: API кілті ортадан жүктелді.",
        dsx_tui::Language::Chinese => "🔑 系统日志: API 密钥已从环境变量加载。",
        dsx_tui::Language::English => "🔑 System Log: API Key is loaded from environment.",
    }
}

fn clear_notice(lang: dsx_tui::Language) -> &'static str {
    match lang {
        dsx_tui::Language::Russian => "🧹 Системный лог: История чата очищена.",
        dsx_tui::Language::Kazakh => "🧹 Жүйелік журнал: Чат тарихы тазартылды.",
        dsx_tui::Language::Chinese => "🧹 系统日志: 当前会话聊天历史记录已清除。",
        dsx_tui::Language::English => "🧹 System Log: Chat history cleared.",
    }
}
