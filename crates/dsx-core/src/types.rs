/// Session identity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

/// Model routing decision
#[derive(Debug, Clone)]
pub enum ModelRoute {
    /// DeepSeek V4 Pro with thinking=max
    ProMax,
    /// DeepSeek V4 Pro with thinking=high
    ProHigh,
    /// DeepSeek V4 Flash, non-thinking
    Flash,
    /// DeepSeek V4 Flash with thinking
    FlashThinking,
}

/// Permission mode for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermissionMode {
    /// Read-only: no edits, no commands.
    ReadOnly,
    /// Plan-only: model can read files and propose plans, but no edits/commands.
    PlanOnly,
    /// Ask before every edit and command (default).
    Ask,
    /// Auto-approve low-risk operations, ask for medium, deny high/blocked.
    AutoApprove,
    /// Auto-approve everything except blocked operations.
    Yolo,
}

impl PermissionMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "readonly" | "read-only" | "ro" => Some(Self::ReadOnly),
            "planonly" | "plan-only" | "plan" => Some(Self::PlanOnly),
            "ask" => Some(Self::Ask),
            "auto" | "auto-approve" | "autoapprove" => Some(Self::AutoApprove),
            "yolo" => Some(Self::Yolo),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "readonly",
            Self::PlanOnly => "plan-only",
            Self::Ask => "ask",
            Self::AutoApprove => "auto",
            Self::Yolo => "yolo",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::ReadOnly => "Read-only: no edits or commands",
            Self::PlanOnly => "Plan-only: read files, propose plans",
            Self::Ask => "Ask before edits and commands",
            Self::AutoApprove => "Auto low-risk, ask medium, deny high",
            Self::Yolo => "Auto everything except blocked",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::ReadOnly,
            Self::PlanOnly,
            Self::Ask,
            Self::AutoApprove,
            Self::Yolo,
        ]
    }
}

/// Risk level for a command or edit
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum RiskLevel {
    Read,
    Low,
    Medium,
    High,
    Blocked,
}
