//! Rolling transcript compaction for long ReAct runs.

use crate::tool_defs::summarize_tool_results;
use crate::types::ToolResult;
use dsx_provider::types::Message;

const SUMMARY_PREFIX: &str = "Rolling transcript summary:";
const COMPACT_AFTER_MESSAGES: usize = 14;
const RECENT_MESSAGE_WINDOW: usize = 6;
const TASK_STATE_CAPSULE_CHARS: usize = 2_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionStats {
    pub removed_messages: usize,
    pub retained_messages: usize,
    pub estimated_tokens_saved: usize,
}

pub fn compact_messages(
    messages: &mut Vec<Message>,
    tool_results: &[ToolResult],
) -> Option<CompactionStats> {
    compact_messages_with_task_state(messages, tool_results, "")
}

pub fn compact_messages_with_task_state(
    messages: &mut Vec<Message>,
    tool_results: &[ToolResult],
    task_state: &str,
) -> Option<CompactionStats> {
    if messages.len() <= COMPACT_AFTER_MESSAGES {
        return None;
    }
    let user_index = messages.iter().position(|msg| msg.role == "user")?;

    let tail_start = safe_tail_start(messages, user_index + 1);
    if tail_start <= user_index + 1 {
        return None;
    }

    let removed_count = tail_start.saturating_sub(user_index + 1);
    let removed_chars = messages[user_index + 1..tail_start]
        .iter()
        .map(message_chars)
        .sum::<usize>();
    let mut compacted = base_prefix(messages, user_index);
    compacted.push(summary_message(removed_count, tool_results, task_state));
    compacted.push(messages[user_index].clone());
    compacted.extend_from_slice(&messages[tail_start..]);
    *messages = compacted;
    Some(CompactionStats {
        removed_messages: removed_count,
        retained_messages: messages.len(),
        estimated_tokens_saved: removed_chars / 4,
    })
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

fn summary_message(removed_count: usize, tool_results: &[ToolResult], task_state: &str) -> Message {
    let capsule = task_state_capsule(task_state);
    let task_line = if capsule.is_empty() {
        "- Task state capsule: unavailable; continue from retained recent messages.".into()
    } else {
        format!("- Task state capsule:\n{capsule}")
    };
    Message {
        role: "system".into(),
        content: Some(format!(
            "{SUMMARY_PREFIX}\n- Older assistant/tool turns compacted: {removed_count} message(s).\n{task_line}\n- Recent tool state: {}\n- Continue from retained recent messages and the task-state capsule.",
            summarize_tool_results(tool_results)
        )),
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }
}

fn task_state_capsule(task_state: &str) -> String {
    let trimmed = task_state.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut capsule: String = trimmed.chars().take(TASK_STATE_CAPSULE_CHARS).collect();
    if trimmed.chars().count() > TASK_STATE_CAPSULE_CHARS {
        capsule.push_str("\n... [task state capsule truncated]");
    }
    capsule
        .lines()
        .map(|line| format!("  {}", line.trim_end()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_summary_message(message: &Message) -> bool {
    message.role == "system"
        && message
            .content
            .as_deref()
            .is_some_and(|content| content.starts_with(SUMMARY_PREFIX))
}

fn message_chars(message: &Message) -> usize {
    let content = message.content.as_deref().unwrap_or("");
    let tool_args = message
        .tool_calls
        .as_ref()
        .map(|calls| calls.iter().map(|call| call.function.arguments.len()).sum())
        .unwrap_or(0);
    content.chars().count() + tool_args
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

        let stats = compact_messages(&mut messages, &tools).unwrap();

        assert_eq!(stats.removed_messages, 18);
        assert_eq!(stats.retained_messages, messages.len());
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

        let _ = compact_messages(&mut messages, &[]);
        let user_index = messages.iter().position(|msg| msg.role == "user").unwrap();

        assert_ne!(messages[user_index + 1].role, "tool");
    }

    #[test]
    fn compaction_summary_keeps_task_state_capsule() {
        let mut messages = vec![msg("system", "rules"), msg("user", "do task")];
        for i in 0..12 {
            messages.push(msg("assistant", &format!("assistant {i}")));
            messages.push(msg("tool", &format!("tool {i}")));
        }
        let tools = vec![tool("read_file", true, "inspected src/main.rs")];
        let task_state = "Compact task brief:\nGoal:\n  build 1234\nDone:\n  inspected files\nPlan:\n  verify\nActive scope:\n  /tmp/sites/1234";

        compact_messages_with_task_state(&mut messages, &tools, task_state).unwrap();
        let summary = messages[1].content.as_deref().unwrap();

        assert!(summary.contains("Task state capsule"));
        assert!(summary.contains("Goal:"));
        assert!(summary.contains("/tmp/sites/1234"));
        assert!(summary.contains("Recent tool state"));
    }

    #[test]
    fn task_state_capsule_is_bounded() {
        let huge_state = format!(
            "Goal:\n  {}\nTAIL",
            "x".repeat(TASK_STATE_CAPSULE_CHARS + 200)
        );

        let capsule = task_state_capsule(&huge_state);

        assert!(capsule.contains("task state capsule truncated"));
        assert!(!capsule.contains("TAIL"));
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
