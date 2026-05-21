//! Tests for TUI keyboard helpers.

#[cfg(test)]
mod tests {
    use crate::tui_keys::active_scope_path;
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
}
