//! DSX Agent — type and config definitions.

use dsx_core::types::RiskLevel;

/// Interactive approval request sent from the agent to the TUI.
pub struct ApprovalRequest {
    pub tool_name: String,
    pub arguments: String,
    pub tx: tokio::sync::oneshot::Sender<bool>,
}

/// Configuration for an agent run.
pub struct AgentConfig {
    pub project_root: std::path::PathBuf,
    pub api_key: String,
    pub api_base: String,
    pub max_iterations: usize,
    pub mode: dsx_core::types::PermissionMode,
    pub approval_tx: Option<tokio::sync::mpsc::UnboundedSender<ApprovalRequest>>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            project_root: std::path::PathBuf::from("."),
            api_key: String::new(),
            api_base: "https://api.deepseek.com".to_string(),
            max_iterations: 15,
            mode: dsx_core::types::PermissionMode::Ask,
            approval_tx: None,
        }
    }
}

/// Result of executing a tool call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub content: String,
    pub success: bool,
    pub risk: RiskLevel,
    pub denied: bool,
}

/// Outcome of an agent run.
#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub answer: Option<String>,
    pub iterations: usize,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub estimated_cost_usd: f64,
    pub tool_results: Vec<ToolResult>,
}
