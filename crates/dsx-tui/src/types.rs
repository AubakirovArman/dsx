//! DSX TUI — type and enum definitions.

/// Active interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Russian,
    Kazakh,
    Chinese,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Russian => "ru",
            Language::Kazakh => "kk",
            Language::Chinese => "zh",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Russian => "Русский",
            Language::Kazakh => "Қазақша",
            Language::Chinese => "中文",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::English, Self::Russian, Self::Kazakh, Self::Chinese]
    }
}

/// A chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct TaskBriefPanel {
    pub goal: String,
    pub done: String,
    pub plan: String,
    pub last_changes: String,
    pub next_step: String,
    pub active_scope: String,
}

impl Default for TaskBriefPanel {
    fn default() -> Self {
        Self {
            goal: "No active task.".into(),
            done: "Idle.".into(),
            plan: "Waiting for the next request.".into(),
            last_changes: "No changes in this run.".into(),
            next_step: "Type a task and press Enter.".into(),
            active_scope: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolTimelineEntry {
    pub name: String,
    pub status: String,
    pub summary: String,
}

/// Agent task running in background.
pub enum AgentTask {
    Idle,
    Running(String),
    Done(String),
    Error(String),
}

/// Events from the agent loop streamed to the TUI.
#[derive(Debug, Clone)]
pub enum AgentStreamEvent {
    /// Reasoning token (thinking mode).
    Reasoning(String),
    /// Content token — appended to the current assistant message.
    ContentToken(String),
    /// A tool call was executed.
    ToolResult {
        name: String,
        success: bool,
        summary: String,
    },
    /// Task completed.
    Done {
        answer: String,
        iterations: usize,
        tokens: u64,
        cost: f64,
    },
    /// Task failed.
    Error(String),
}

/// Interactive approval state inside TUI.
pub struct PendingApproval {
    pub tool_name: String,
    pub arguments: String,
    pub tx: tokio::sync::oneshot::Sender<bool>,
}
