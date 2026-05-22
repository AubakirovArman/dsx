use super::*;
use crate::{FolderNote, ToolTimelineEntry};

#[test]
fn handoff_status_summarizes_context_readiness() {
    let mut app = App::new();

    assert!(handoff_status_text(&app).starts_with("idle;"));

    app.begin_task_scoped("build 1234", "/tmp/sites", "/tmp/sites/1234", true);
    app.folder_notes.clear();
    app.folder_notes.push(FolderNote {
        folder: "1234/".into(),
        summary: "state".into(),
        next_step: "next".into(),
        architecture: "arch".into(),
    });
    app.tool_timeline.push(ToolTimelineEntry {
        name: "read_file".into(),
        status: "ok".into(),
        summary: "inspected".into(),
    });
    app.compaction_events = 1;
    app.estimated_tokens_saved = 340;

    let status = handoff_status_text(&app);

    assert!(status.starts_with("ready;"));
    assert!(status.contains("scope /tmp/sites/1234"));
    assert!(status.contains("notes 1"));
    assert!(status.contains("tools 1"));
    assert!(status.contains("compact 1/~340tok"));

    app.scope_violations = 1;
    assert!(handoff_status_text(&app).starts_with("blocked;"));
}
