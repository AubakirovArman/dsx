//! Tests for TUI keyboard helpers.

#[cfg(test)]
mod tests {
    use crate::tui_keys::{
        active_scope_path, handle_context_key, toggle_context, toggle_settings, toggle_tools,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::sync::{Arc, Mutex};

    #[test]
    fn active_scope_path_prefers_scope_lock() {
        let launch = std::path::Path::new("/tmp/sites");
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        app.lock().unwrap().scope_lock.active_scope = "/tmp/sites/1234".into();

        assert_eq!(active_scope_path(&app, launch), launch.join("1234"));
    }

    #[test]
    fn active_scope_path_falls_back_to_launch_scope() {
        let launch = std::path::Path::new("/tmp/sites");
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));

        assert_eq!(active_scope_path(&app, launch), launch);
    }

    #[test]
    fn tools_view_toggle_hides_other_modal_views() {
        let mut app = dsx_tui::App::new();
        app.show_diff = true;
        app.show_settings = true;

        toggle_tools(&mut app);

        assert!(app.show_tools);
        assert!(!app.show_diff);
        assert!(!app.show_settings);
    }

    #[test]
    fn settings_toggle_hides_tools_view() {
        let mut app = dsx_tui::App::new();
        app.show_tools = true;

        toggle_settings(&mut app);

        assert!(app.show_settings);
        assert!(!app.show_tools);
    }

    #[test]
    fn context_toggle_hides_other_modal_views() {
        let mut app = dsx_tui::App::new();
        app.show_diff = true;
        app.show_tools = true;
        app.show_settings = true;

        toggle_context(&mut app);

        assert!(app.show_context);
        assert!(!app.show_diff);
        assert!(!app.show_tools);
        assert!(!app.show_settings);
    }

    #[test]
    fn context_view_keys_move_folder_focus() {
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        {
            let mut app = app.lock().unwrap();
            app.upsert_folder_note("/tmp/sites/one", "one", "next");
            app.upsert_folder_note("/tmp/sites/two", "two", "next");
        }

        handle_context_key(key(KeyCode::Down), &app);
        assert_eq!(
            app.lock().unwrap().focused_folder_note().unwrap().folder,
            "one/"
        );

        handle_context_key(key(KeyCode::Up), &app);
        assert_eq!(
            app.lock().unwrap().focused_folder_note().unwrap().folder,
            "two/"
        );
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
}
