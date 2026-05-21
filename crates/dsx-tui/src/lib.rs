//! DSX TUI — ratatui-based interactive terminal workspace.

pub mod draw;
pub mod draw_settings;
pub mod i18n;
pub mod types;

pub use types::{AgentStreamEvent, AgentTask, ChatMessage, Language, PendingApproval};

/// Shared app state.
pub struct App {
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub mode: String,
    pub model: String,
    pub tokens: u64,
    pub cost: f64,
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
                self.add_message("tool", &format!("{status} {name} — {short}"));
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
            }
            AgentStreamEvent::Error(err) => {
                self.add_message("error", err);
                self.current_reasoning.clear();
                self.agent_task = AgentTask::Error(err.clone());
            }
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ChatMessage {
            role: role.into(),
            content: content.into(),
        });
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
