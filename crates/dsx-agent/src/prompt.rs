//! Initial prompt assembly for state-first agent runs.

use dsx_provider::types::Message;

pub fn build_start_messages(
    system_prompt: impl Into<String>,
    scope_note: &str,
    project_context: &str,
    task_state: &str,
    project_instructions: Option<&str>,
    clean_task: &str,
) -> Vec<Message> {
    let mut messages = vec![
        msg("system", system_prompt),
        msg(
            "system",
            format!("{scope_note}\n\nActive-scope project context:\n{project_context}"),
        ),
        msg("system", context_capsule(task_state)),
    ];
    if let Some(instructions) = project_instructions {
        messages.push(msg(
            "system",
            format!("Project-specific instructions:\n{instructions}"),
        ));
    }
    messages.push(msg("user", clean_task));
    messages
}

pub fn context_capsule(task_state: &str) -> String {
    format!(
        "Context capsule:\n\
         - Use this compact task state instead of previous chat history.\n\
         - Treat active scope, constraints, plan, done, and next step as authoritative.\n\n{}",
        task_state.trim()
    )
}

fn msg(role: &str, content: impl Into<String>) -> Message {
    Message {
        role: role.into(),
        content: Some(content.into()),
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_messages_include_context_capsule_without_history() {
        let messages = build_start_messages(
            "lead",
            "scope note",
            "Files:\n  src/main.rs",
            "Goal:\n  build\nActive scope:\n  /tmp/sites/1234",
            None,
            "build",
        );

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[3].role, "user");
        assert!(
            messages[2]
                .content
                .as_deref()
                .unwrap()
                .contains("instead of previous chat history")
        );
        assert!(
            messages[2]
                .content
                .as_deref()
                .unwrap()
                .contains("/tmp/sites/1234")
        );
    }

    #[test]
    fn start_messages_include_project_instructions_before_user() {
        let messages =
            build_start_messages("lead", "scope", "ctx", "state", Some("stay scoped"), "task");

        assert_eq!(messages.len(), 5);
        assert!(
            messages[3]
                .content
                .as_deref()
                .unwrap()
                .contains("Project-specific instructions")
        );
        assert_eq!(messages[4].role, "user");
    }
}
