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
                summary: format!("requested {}", tc.name),
            }
        }
        dsx_provider::streaming::StreamEvent::ToolResult {
            name,
            success,
            summary,
        } => dsx_tui::AgentStreamEvent::ToolResult {
            name: name.clone(),
            success: *success,
            summary: summary.clone(),
        },
        dsx_provider::streaming::StreamEvent::Finish { .. } => {
            dsx_tui::AgentStreamEvent::Reasoning(String::new())
        }
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
