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
