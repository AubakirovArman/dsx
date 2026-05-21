//! DSX TUI — ratatui-based interactive terminal workspace.

#[cfg(test)]
mod app_tests;
pub mod draw;
pub mod draw_chat;
pub mod draw_input;
pub mod draw_settings;
pub mod draw_status;
pub mod draw_workflow;
pub mod i18n;
pub mod types;

pub use types::{
    AgentStreamEvent, AgentTask, ChatMessage, Language, PendingApproval, TaskBriefPanel,
    ToolTimelineEntry,
};

/// Shared app state.
pub struct App {
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub mode: String,
    pub model: String,
    pub tokens: u64,
    pub cost: f64,
    pub run_start_tokens: u64,
    pub run_start_cost: f64,
    pub agent_task: AgentTask,
    pub cursor_pos: usize,
    pub scroll_offset: u16,
    pub file_tree: Vec<String>,
    pub show_file_tree: bool,
    pub pending_approval: Option<PendingApproval>,
    pub current_reasoning: String,
    pub show_settings: bool,
    pub settings_cursor: usize,
    pub show_diff: bool,
    pub current_diff: String,
    pub lang: Language,
    pub api_base: String,
    pub api_key: String,
    pub budget_status: String,
    pub active_run_id: Option<u64>,
    pub active_ledger_id: Option<String>,
    pub next_run_id: u64,
    pub agent_abort: Option<tokio::task::AbortHandle>,
    pub compaction_events: u64,
    pub compacted_messages: u64,
    pub estimated_tokens_saved: u64,
    pub task_brief: TaskBriefPanel,
    pub tool_timeline: Vec<ToolTimelineEntry>,
}

impl App {
    pub fn new() -> Self {
        let initial_lang = Language::English;
        let initial_msg = match initial_lang {
            Language::Russian => {
                "DSX Code — ИИ-агент для кодинга на базе DeepSeek V4.\nВведите задачу и нажмите Enter. Ctrl+C для выхода."
            }
            Language::Kazakh => {
                "DSX Code — DeepSeek V4 негізіндегі ИИ кодинг агенті.\nТапсырманы енгізіп, Enter басыңыз. Шығу үшін Ctrl+C басыңыз."
            }
            Language::Chinese => {
                "DSX Code — 基于 DeepSeek V4 的 AI 编程助手。\n输入任务并按 Enter 回车。按 Ctrl+C 退出。"
            }
            Language::English => {
                "DSX Code — DeepSeek V4 coding agent.\nType a task and press Enter. Ctrl+C to quit."
            }
        };

        Self {
            input: String::new(),
            messages: vec![ChatMessage {
                role: "system".into(),
                content: initial_msg.into(),
            }],
            mode: "ask".into(),
            model: "v4-pro".into(),
            tokens: 0,
            cost: 0.0,
            run_start_tokens: 0,
            run_start_cost: 0.0,
            agent_task: AgentTask::Idle,
            cursor_pos: 0,
            scroll_offset: 0,
            file_tree: Vec::new(),
            show_file_tree: false,
            pending_approval: None,
            current_reasoning: String::new(),
            show_settings: false,
            settings_cursor: 0,
            show_diff: false,
            current_diff: String::new(),
            lang: initial_lang,
            api_base: "https://api.deepseek.com".to_string(),
            api_key: String::new(),
            budget_status: String::new(),
            active_run_id: None,
            active_ledger_id: None,
            next_run_id: 0,
            agent_abort: None,
            compaction_events: 0,
            compacted_messages: 0,
            estimated_tokens_saved: 0,
            task_brief: TaskBriefPanel::default(),
            tool_timeline: Vec::new(),
        }
    }

    /// Retrieve localized text constant for key.
    pub fn tr(&self, key: &str) -> &'static str {
        i18n::tr(self.lang, key)
    }

    /// Process a streaming event from the agent.
    pub fn handle_stream_event(&mut self, event: &AgentStreamEvent) {
        match event {
            AgentStreamEvent::Reasoning(r) => {
                self.current_reasoning.push_str(r);
            }
            AgentStreamEvent::ContentToken(token) => {
                // Clear reasoning on first content token so the reasoning panel closes
                if !self.current_reasoning.is_empty() {
                    self.current_reasoning.clear();
                }
                // Append to the last assistant message, or create one
                if let Some(last) = self.messages.last_mut()
                    && last.role == "assistant"
                {
                    last.content.push_str(token);
                    return;
                }
                self.messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: token.clone(),
                });
            }
            AgentStreamEvent::ToolResult {
                name,
                success,
                summary,
            } => {
                let status = if *success { "✓" } else { "✗" };
                let short: String = summary.chars().take(150).collect();
                self.push_tool_event(name, *success, &short);
                self.add_message("tool", &format!("{status} {name} — {short}"));
            }
            AgentStreamEvent::TranscriptCompact {
                removed_messages,
                retained_messages,
                estimated_tokens_saved,
            } => {
                self.compaction_events += 1;
                self.compacted_messages += *removed_messages as u64;
                self.estimated_tokens_saved += *estimated_tokens_saved as u64;
                let summary = format!(
                    "{removed_messages} msg compacted, ~{estimated_tokens_saved} tok saved, {retained_messages} retained"
                );
                self.push_tool_event("context_compact", true, &summary);
                self.add_message("system", &format!("Context compacted: {summary}"));
            }
            AgentStreamEvent::Done {
                answer: _ans,
                iterations,
                tokens,
                cost,
            } => {
                self.tokens += tokens;
                self.cost += cost;
                self.current_reasoning.clear();
                self.agent_task =
                    AgentTask::Done(format!("{} iterations, ${:.4}", iterations, cost));
                self.task_brief.done = format!("Completed in {iterations} iteration(s).");
                self.task_brief.last_changes = "Final assistant response recorded.".into();
                self.task_brief.next_step = "Review result or enter the next task.".into();
            }
            AgentStreamEvent::Error(err) => {
                self.add_message("error", err);
                self.current_reasoning.clear();
                self.agent_task = AgentTask::Error(err.clone());
                self.task_brief.done = "Run failed.".into();
                self.task_brief.last_changes = err.chars().take(220).collect();
                self.task_brief.next_step =
                    "Inspect the error and retry with a narrower task.".into();
            }
        }
    }

    pub fn begin_task(&mut self, task: &str, active_scope: &str) {
        self.run_start_tokens = self.tokens;
        self.run_start_cost = self.cost;
        self.task_brief = TaskBriefPanel {
            goal: truncate_chars(task, 260),
            done: "Task accepted; context brief prepared.".into(),
            plan: "1. Stay inside active scope\n2. Inspect only needed files\n3. Apply scoped changes\n4. Verify and summarize".into(),
            last_changes: "No tool result yet.".into(),
            next_step: "Waiting for first model/tool event.".into(),
            active_scope: active_scope.into(),
        };
        self.tool_timeline.clear();
        self.compaction_events = 0;
        self.compacted_messages = 0;
        self.estimated_tokens_saved = 0;
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ChatMessage {
            role: role.into(),
            content: content.into(),
        });
    }

    fn push_tool_event(&mut self, name: &str, success: bool, summary: &str) {
        let status = if success { "ok" } else { "failed" };
        self.tool_timeline.push(ToolTimelineEntry {
            name: name.into(),
            status: status.into(),
            summary: summary.into(),
        });
        if self.tool_timeline.len() > 20 {
            let overflow = self.tool_timeline.len() - 20;
            self.tool_timeline.drain(0..overflow);
        }
        self.task_brief.done = format!("Tool {name} finished with status {status}.");
        self.task_brief.last_changes = summary.into();
        self.task_brief.next_step = if success {
            "Continue from the latest tool result.".into()
        } else {
            "Review failed tool output before continuing.".into()
        };
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_chars(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        text.push_str("...");
    }
    text
}
