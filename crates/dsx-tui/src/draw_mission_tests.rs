use super::*;
use crate::{App, ToolTimelineEntry};

#[test]
fn mission_tool_counts_tracks_visible_statuses() {
    let mut app = App::new();
    for status in ["ok", "failed", "blocked", "ok"] {
        app.tool_timeline.push(ToolTimelineEntry {
            name: "tool".into(),
            status: status.into(),
            summary: "summary".into(),
        });
    }

    assert_eq!(mission_tool_counts(&app), (2, 1, 1));
}

#[test]
fn mission_scope_guard_summarizes_last_block() {
    let mut app = App::new();
    app.scope_violations = 2;
    app.last_scope_violation = "read_file denied outside active scope".into();

    assert!(scope_guard_text(&app).contains("2 blocked"));
    assert!(scope_guard_text(&app).contains("read_file"));
}
