//! Conversation message helpers for DeepSeek thinking-mode turns.

use dsx_provider::types::{Message, ThinkingConfig, ToolCall};

pub(crate) fn thinking_config(enabled: bool) -> ThinkingConfig {
    ThinkingConfig {
        type_: if enabled { "enabled" } else { "disabled" }.into(),
    }
}

pub(crate) fn assistant_message(
    content: &str,
    tool_calls: Vec<ToolCall>,
    reasoning: &str,
    include_reasoning: bool,
) -> Message {
    Message {
        role: "assistant".into(),
        content: if include_reasoning {
            Some(content.into())
        } else {
            non_empty(content)
        },
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        tool_call_id: None,
        reasoning_content: include_reasoning.then(|| reasoning.into()),
    }
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsx_provider::types::FunctionCall;

    #[test]
    fn thinking_config_explicitly_disables_non_thinking_routes() {
        assert_eq!(thinking_config(false).type_, "disabled");
        assert_eq!(thinking_config(true).type_, "enabled");
    }

    #[test]
    fn assistant_message_keeps_reasoning_for_thinking_turns() {
        let message = assistant_message("answer", Vec::new(), "chain", true);

        assert_eq!(message.content.as_deref(), Some("answer"));
        assert_eq!(message.reasoning_content.as_deref(), Some("chain"));
    }

    #[test]
    fn assistant_message_skips_reasoning_when_not_thinking() {
        let message = assistant_message("answer", Vec::new(), "chain", false);

        assert!(message.reasoning_content.is_none());
    }

    #[test]
    fn assistant_message_keeps_empty_reasoning_for_thinking_turns() {
        let message = assistant_message("answer", Vec::new(), "", true);

        assert_eq!(message.content.as_deref(), Some("answer"));
        assert_eq!(message.reasoning_content.as_deref(), Some(""));
    }

    #[test]
    fn assistant_message_keeps_empty_content_for_thinking_turns() {
        let message = assistant_message("", Vec::new(), "chain", true);

        assert_eq!(message.content.as_deref(), Some(""));
        assert_eq!(message.reasoning_content.as_deref(), Some("chain"));
    }

    #[test]
    fn assistant_message_keeps_tool_calls_with_reasoning() {
        let message = assistant_message(
            "",
            vec![ToolCall {
                id: "call_1".into(),
                type_: "function".into(),
                function: FunctionCall {
                    name: "read_file".into(),
                    arguments: "{}".into(),
                },
            }],
            "need file",
            true,
        );

        assert_eq!(message.content.as_deref(), Some(""));
        assert_eq!(message.tool_calls.unwrap().len(), 1);
        assert_eq!(message.reasoning_content.as_deref(), Some("need file"));
    }
}
