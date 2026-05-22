//! DSX TUI — ratatui-based interactive terminal workspace.

#[cfg(test)]
mod app_tests;
pub mod draw;
pub mod draw_chat;
pub mod draw_context;
pub mod draw_input;
pub mod draw_mission;
pub mod draw_mission_state;
pub mod draw_panes;
pub mod draw_settings;
pub mod draw_status;
pub mod draw_tools;
pub mod draw_workflow;
pub mod draw_workflow_panels;
pub mod folder_notes;
pub mod i18n;
pub mod stream_events;
pub mod types;

pub use types::{
    AgentStreamEvent, AgentTask, ChatMessage, FolderNote, Language, PendingApproval,
    ScopeLockPanel, TaskBriefPanel, ToolTimelineEntry,
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
    pub show_tools: bool,
    pub show_context: bool,
    pub show_mission: bool,
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
    pub scope_violations: u64,
    pub last_scope_violation: String,
    pub task_brief: TaskBriefPanel,
    pub scope_lock: ScopeLockPanel,
    pub folder_notes: Vec<FolderNote>,
    pub folder_note_cursor: usize,
    pub tool_timeline: Vec<ToolTimelineEntry>,
    pub allow_wide_scope: bool,
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
            show_tools: false,
            show_context: false,
            show_mission: false,
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
            scope_violations: 0,
            last_scope_violation: String::new(),
            task_brief: TaskBriefPanel::default(),
            scope_lock: ScopeLockPanel::default(),
            folder_notes: Vec::new(),
            folder_note_cursor: 0,
            tool_timeline: Vec::new(),
            allow_wide_scope: false,
        }
    }

    /// Retrieve localized text constant for key.
    pub fn tr(&self, key: &str) -> &'static str {
        i18n::tr(self.lang, key)
    }

    pub fn begin_task(&mut self, task: &str, active_scope: &str) {
        self.begin_task_scoped(task, active_scope, active_scope, false);
    }

    pub fn begin_task_scoped(
        &mut self,
        task: &str,
        launch_scope: &str,
        active_scope: &str,
        narrowed: bool,
    ) {
        self.run_start_tokens = self.tokens;
        self.run_start_cost = self.cost;
        self.task_brief = TaskBriefPanel {
            goal: truncate_chars(task, 260),
            done: "Task accepted; context brief prepared.".into(),
            plan: "1. Stay inside active scope\n2. Inspect only needed files\n3. Apply scoped changes\n4. Verify and summarize".into(),
            last_changes: "No tool result yet.".into(),
            next_step: "Waiting for first model/tool event.".into(),
            active_scope: active_scope.into(),
            constraints: "Active scope is a hard boundary; keep source files <= 300 lines.".into(),
            architecture: "Architecture will be refreshed during context preflight.".into(),
        };
        let status = if narrowed { "Narrowed" } else { "Wide" };
        self.scope_lock = ScopeLockPanel {
            launch_scope: launch_scope.into(),
            active_scope: active_scope.into(),
            status: status.into(),
            reason: if narrowed {
                "Task selected a subfolder; tools and indexing are locked there.".into()
            } else {
                "No explicit subfolder was selected; launch workspace is active.".into()
            },
            warning: if narrowed {
                String::new()
            } else {
                "Review the task if you expected a narrower folder like ./1234.".into()
            },
        };
        let next_step = self.task_brief.next_step.clone();
        self.upsert_folder_note(active_scope, "Task accepted in this folder.", &next_step);
        self.add_message(
            "system",
            &scope_contract_message(launch_scope, active_scope, status),
        );
        self.tool_timeline.clear();
        self.compaction_events = 0;
        self.compacted_messages = 0;
        self.estimated_tokens_saved = 0;
        self.scope_violations = 0;
        self.last_scope_violation.clear();
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

fn scope_contract_message(launch_scope: &str, active_scope: &str, status: &str) -> String {
    format!(
        "Scope contract: launch={} -> active={} ({status}); tools, indexing, and memory stay inside active scope.",
        launch_scope, active_scope
    )
}

fn truncate_chars(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        text.push_str("...");
    }
    text
}
