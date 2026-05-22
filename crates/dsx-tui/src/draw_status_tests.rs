use super::*;

#[test]
fn scope_badge_shows_narrow_active_folder() {
    let mut app = App::new();
    app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites/1234", true);

    assert_eq!(scope_badge(&app), "narrow:1234");
}

#[test]
fn scope_badge_marks_blocked_scope() {
    let mut app = App::new();
    app.scope_lock.active_scope = "/tmp/sites".into();
    app.scope_lock.status = "Blocked".into();

    assert_eq!(scope_badge(&app), "blocked:sites");
}
