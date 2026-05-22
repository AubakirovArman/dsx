//! Conversion from provider stream events into TUI events.

pub fn convert_event(ev: &dsx_provider::streaming::StreamEvent) -> dsx_tui::AgentStreamEvent {
    match ev {
        dsx_provider::streaming::StreamEvent::Reasoning(r) => {
            dsx_tui::AgentStreamEvent::Reasoning(r.clone())
        }
        dsx_provider::streaming::StreamEvent::Content(c) => {
            dsx_tui::AgentStreamEvent::ContentToken(c.clone())
        }
        dsx_provider::streaming::StreamEvent::ToolCall(tc) => {
            dsx_tui::AgentStreamEvent::ToolResult {
                name: tc.name.clone(),
                success: true,
                denied: false,
                risk: "Read".into(),
                summary: format!("requested {}", tc.name),
            }
        }
        dsx_provider::streaming::StreamEvent::ToolResult {
            name,
            success,
            denied,
            risk,
            summary,
        } => dsx_tui::AgentStreamEvent::ToolResult {
            name: name.clone(),
            success: *success,
            denied: *denied,
            risk: risk.clone(),
            summary: summary.clone(),
        },
        dsx_provider::streaming::StreamEvent::TranscriptCompact {
            removed_messages,
            retained_messages,
            estimated_tokens_saved,
        } => dsx_tui::AgentStreamEvent::TranscriptCompact {
            removed_messages: *removed_messages,
            retained_messages: *retained_messages,
            estimated_tokens_saved: *estimated_tokens_saved,
        },
        dsx_provider::streaming::StreamEvent::Finish { usage, .. } => usage
            .as_ref()
            .map(|usage| dsx_tui::AgentStreamEvent::Usage {
                prompt_tokens: usage.prompt_tokens as u64,
                completion_tokens: usage.completion_tokens as u64,
                reasoning_tokens: usage.reasoning_tokens.unwrap_or(0) as u64,
                total_tokens: usage.total_tokens as u64,
            })
            .unwrap_or_else(|| dsx_tui::AgentStreamEvent::Reasoning(String::new())),
        dsx_provider::streaming::StreamEvent::Done {
            answer,
            iterations,
            tokens,
            cost,
        } => dsx_tui::AgentStreamEvent::Done {
            answer: answer.clone(),
            iterations: *iterations,
            tokens: *tokens,
            cost: *cost,
        },
        dsx_provider::streaming::StreamEvent::Error(err) => {
            dsx_tui::AgentStreamEvent::Error(err.clone())
        }
    }
}
