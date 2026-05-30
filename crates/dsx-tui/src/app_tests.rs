//! Tests for TUI app event handling.

#[cfg(test)]
mod tests {
    use crate::{AgentStreamEvent, App, FolderNote, Language};

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
        assert_eq!(app.run_budget.used_tokens, 0);
        assert_eq!(app.run_budget.status, "running");
    }

    #[test]
    fn stream_usage_updates_run_budget_without_session_tokens() {
        let mut app = App::new();
        app.run_budget.max_tokens = 100;
        app.run_budget.max_cost_usd = 2.0;

        app.handle_stream_event(&AgentStreamEvent::Usage {
            prompt_tokens: 80,
            completion_tokens: 5,
            reasoning_tokens: 5,
            total_tokens: 85,
        });

        assert_eq!(app.tokens, 0);
        assert_eq!(app.run_budget.used_tokens, 90);
        assert_eq!(app.run_budget.status, "near");
        assert!(app.run_budget.last_update.contains("reasoning 5"));
    }

    #[test]
    fn begin_task_scoped_updates_visible_scope_lock() {
        let mut app = App::new();

        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites/1234", true);

        assert_eq!(app.scope_lock.launch_scope, "/tmp/sites");
        assert_eq!(app.scope_lock.active_scope, "/tmp/sites/1234");
        assert_eq!(app.scope_lock.status, "Narrowed");
        assert!(app.scope_lock.warning.is_empty());
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("Scope contract")
                    && msg.content.contains("/tmp/sites/1234"))
        );
    }

    #[test]
    fn begin_task_scoped_warns_when_scope_is_wide() {
        let mut app = App::new();

        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites", false);

        assert_eq!(app.scope_lock.status, "Wide");
        assert!(app.scope_lock.warning.contains("narrower folder"));
    }

    #[test]
    fn tool_result_updates_visible_folder_note() {
        let mut app = App::new();
        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites/1234", true);

        app.handle_stream_event(&AgentStreamEvent::ToolResult {
            name: "read_file".into(),
            success: true,
            denied: false,
            risk: "Read".into(),
            summary: "inspected src/main.rs".into(),
        });

        let note = app
            .folder_notes
            .iter()
            .find(|note| note.folder == "1234/")
            .unwrap();
        assert_eq!(note.summary, "inspected src/main.rs");
        assert!(note.next_step.contains("Continue"));
        assert!(note.architecture.contains("active project folder"));
    }

    #[test]
    fn blocked_scope_tool_result_updates_visible_guardrails() {
        let mut app = App::new();
        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites/1234", true);

        app.handle_stream_event(&AgentStreamEvent::ToolResult {
            name: "read_file".into(),
            success: false,
            denied: true,
            risk: "Blocked".into(),
            summary: "Path denied by active scope: path traversal blocked".into(),
        });

        assert_eq!(app.scope_violations, 1);
        assert!(app.last_scope_violation.contains("read_file"));
        assert!(app.scope_lock.warning.contains("blocked scope escape"));
        assert_eq!(app.tool_timeline.last().unwrap().status, "blocked");
        assert!(app.task_brief.next_step.contains("active scope"));
    }

    #[test]
    fn folder_note_cap_keeps_new_active_scope() {
        let mut app = App::new();
        for idx in 0..12 {
            app.upsert_folder_note(&format!("/tmp/sites/p{idx}"), "old", "next");
        }

        app.upsert_folder_note("/tmp/sites/new", "fresh", "verify");

        assert_eq!(app.folder_notes.len(), 12);
        assert!(app.folder_notes.iter().any(|note| note.folder == "new/"));
        assert!(!app.folder_notes.iter().any(|note| note.folder == "p0/"));
    }

    #[test]
    fn folder_note_focus_wraps_and_tracks_updates() {
        let mut app = App::new();
        app.upsert_folder_note("/tmp/sites/one", "one", "next");
        app.upsert_folder_note("/tmp/sites/two", "two", "next");

        assert_eq!(app.focused_folder_note().unwrap().folder, "two/");

        app.select_next_folder_note();
        assert_eq!(app.focused_folder_note().unwrap().folder, "one/");

        app.select_previous_folder_note();
        assert_eq!(app.focused_folder_note().unwrap().folder, "two/");

        app.upsert_folder_note("/tmp/sites/one", "updated", "verify");
        let note = app.focused_folder_note().unwrap();
        assert_eq!(note.folder, "one/");
        assert_eq!(note.summary, "updated");
    }

    #[test]
    fn focused_folder_scope_resolves_inside_launch_scope() {
        let mut app = App::new();
        app.scope_lock.launch_scope = "/tmp/sites".into();
        app.upsert_folder_note("/tmp/sites/1234", "state", "next");

        assert_eq!(app.focused_folder_scope().unwrap(), "/tmp/sites/1234");

        app.set_folder_notes(vec![note("../outside/")]);
        assert!(app.focused_folder_scope().is_none());
    }

    #[test]
    fn draft_focused_scope_task_updates_input_or_reports_unsafe_note() {
        let mut app = App::new();
        app.show_context = true;
        app.scope_lock.launch_scope = "/tmp/sites".into();
        app.upsert_folder_note("/tmp/sites/1234", "state", "next");
        app.input = "polish UI".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: polish UI");
        assert_eq!(app.cursor_pos, app.input.chars().count());
        assert!(!app.show_context);

        app.show_context = true;
        app.input = "use folder old only: polish UI".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: polish UI");

        app.show_context = true;
        app.input = "используй папку old только: отполируй UI".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: отполируй UI");

        app.show_context = true;
        app.input = "используй папку old только отполируй UI".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: отполируй UI");

        app.show_context = true;
        app.input = "use folder old only polish UI".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: polish UI");

        app.show_context = true;
        app.input = "используй текущий воркспейс только: проверь сборку".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: проверь сборку");

        app.show_context = true;
        app.lang = Language::Russian;
        app.input = "отполируй UI".into();

        assert!(app.draft_focused_scope_task());
        assert_eq!(app.input, "используй папку 1234 только: отполируй UI");

        app.show_context = true;
        app.input = "keep".into();
        app.set_folder_notes(vec![note("../outside/")]);

        assert!(!app.draft_focused_scope_task());
        assert_eq!(app.input, "keep");
        assert!(app.show_context);
        assert!(app.messages.iter().any(|msg| msg.role == "error"));
    }

    fn note(folder: &str) -> FolderNote {
        FolderNote {
            folder: folder.into(),
            summary: "state".into(),
            next_step: "next".into(),
            architecture: "arch".into(),
        }
    }
}
