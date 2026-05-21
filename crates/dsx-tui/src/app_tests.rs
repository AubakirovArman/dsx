//! Tests for TUI app event handling.

#[cfg(test)]
mod tests {
    use crate::{AgentStreamEvent, App};

    #[test]
    fn transcript_compaction_updates_visible_telemetry() {
        let mut app = App::new();

        app.handle_stream_event(&AgentStreamEvent::TranscriptCompact {
            removed_messages: 12,
            retained_messages: 9,
            estimated_tokens_saved: 340,
        });

        assert_eq!(app.compaction_events, 1);
        assert_eq!(app.compacted_messages, 12);
        assert_eq!(app.estimated_tokens_saved, 340);
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Context compacted"))
        );
        assert!(
            app.tool_timeline
                .iter()
                .any(|entry| entry.name == "context_compact")
        );
    }

    #[test]
    fn begin_task_resets_compaction_counters() {
        let mut app = App::new();
        app.compaction_events = 2;
        app.compacted_messages = 20;
        app.estimated_tokens_saved = 500;

        app.begin_task("next", "/tmp/project");

        assert_eq!(app.compaction_events, 0);
        assert_eq!(app.compacted_messages, 0);
        assert_eq!(app.estimated_tokens_saved, 0);
    }

    #[test]
    fn begin_task_scoped_updates_visible_scope_lock() {
        let mut app = App::new();

        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites/1234", true);

        assert_eq!(app.scope_lock.launch_scope, "/tmp/sites");
        assert_eq!(app.scope_lock.active_scope, "/tmp/sites/1234");
        assert_eq!(app.scope_lock.status, "Narrowed");
        assert!(app.scope_lock.warning.is_empty());
    }

    #[test]
    fn begin_task_scoped_warns_when_scope_is_wide() {
        let mut app = App::new();

        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites", false);

        assert_eq!(app.scope_lock.status, "Wide");
        assert!(app.scope_lock.warning.contains("narrower folder"));
    }
}
