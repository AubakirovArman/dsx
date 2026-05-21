//! Rolling transcript compaction for long ReAct runs.

use crate::tool_defs::summarize_tool_results;
use crate::types::ToolResult;
use dsx_provider::types::Message;

const SUMMARY_PREFIX: &str = "Rolling transcript summary:";
const COMPACT_AFTER_MESSAGES: usize = 18;
const RECENT_MESSAGE_WINDOW: usize = 8;

pub fn compact_messages(messages: &mut Vec<Message>, tool_results: &[ToolResult]) -> bool {
    if messages.len() <= COMPACT_AFTER_MESSAGES {
        return false;
    }
    let Some(user_index) = messages.iter().position(|msg| msg.role == "user") else {
        return false;
    };

    let tail_start = safe_tail_start(messages, user_index + 1);
    if tail_start <= user_index + 1 {
        return false;
    }

    let removed_count = tail_start.saturating_sub(user_index + 1);
    let mut compacted = base_prefix(messages, user_index);
    compacted.push(summary_message(removed_count, tool_results));
    compacted.push(messages[user_index].clone());
    compacted.extend_from_slice(&messages[tail_start..]);
    *messages = compacted;
    true
}

fn base_prefix(messages: &[Message], user_index: usize) -> Vec<Message> {
    messages[..user_index]
        .iter()
        .filter(|msg| !is_summary_message(msg))
        .cloned()
        .collect()
}

fn safe_tail_start(messages: &[Message], prefix_len: usize) -> usize {
    let mut start = messages
        .len()
        .saturating_sub(RECENT_MESSAGE_WINDOW)
        .max(prefix_len);
    while start > prefix_len && messages[start].role == "tool" {
        start -= 1;
    }
    start
}

fn summary_message(removed_count: usize, tool_results: &[ToolResult]) -> Message {
    Message {
        role: "system".into(),
        content: Some(format!(
            "{SUMMARY_PREFIX}\n- Older assistant/tool turns compacted: {removed_count} message(s).\n- Recent tool state: {}\n- Continue from retained recent messages and the active task brief.",
            summarize_tool_results(tool_results)
        )),
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }
}

fn is_summary_message(message: &Message) -> bool {
    message.role == "system"
        && message
            .content
            .as_deref()
            .is_some_and(|content| content.starts_with(SUMMARY_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsx_core::types::RiskLevel;

    #[test]
    fn compacts_old_turns_and_keeps_recent_tail() {
        let mut messages = vec![msg("system", "rules"), msg("user", "do task")];
        for i in 0..12 {
            messages.push(msg("assistant", &format!("assistant {i}")));
            messages.push(msg("tool", &format!("tool {i}")));
        }
        let tools = vec![tool("read_file", true, "ok")];

        assert!(compact_messages(&mut messages, &tools));

        assert_eq!(messages[0].role, "system");
        assert!(
            messages[1]
                .content
                .as_deref()
                .unwrap()
                .starts_with(SUMMARY_PREFIX)
        );
        assert_eq!(messages[2].role, "user");
        assert!(messages.len() < 14);
        assert!(
            messages
                .iter()
                .any(|msg| msg.content.as_deref() == Some("tool 11"))
        );
    }

    #[test]
    fn tail_never_starts_with_orphan_tool_message() {
        let mut messages = vec![msg("system", "rules"), msg("user", "do task")];
        for i in 0..10 {
            messages.push(msg("assistant", &format!("assistant {i}")));
            messages.push(msg("tool", &format!("tool {i}")));
        }

        compact_messages(&mut messages, &[]);
        let user_index = messages.iter().position(|msg| msg.role == "user").unwrap();

        assert_ne!(messages[user_index + 1].role, "tool");
    }

    fn msg(role: &str, content: &str) -> Message {
        Message {
            role: role.into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    fn tool(name: &str, success: bool, content: &str) -> ToolResult {
        ToolResult {
            tool_call_id: "call_1".into(),
            name: name.into(),
            content: content.into(),
            success,
            risk: RiskLevel::Read,
            denied: false,
        }
    }
}
