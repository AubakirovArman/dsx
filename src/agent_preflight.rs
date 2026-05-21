//! Agent-start dry-run diagnostics.

use std::path::Path;

pub(crate) struct AgentPreflight {
    pub(crate) task: String,
    pub(crate) launch: String,
    pub(crate) active: String,
    pub(crate) narrowed: bool,
    pub(crate) active_exists: bool,
    pub(crate) allow_wide_scope: bool,
    pub(crate) policy_source: String,
    pub(crate) suggested_scopes: Vec<String>,
    pub(crate) blocker: Option<String>,
    pub(crate) reason: String,
}

impl AgentPreflight {
    pub(crate) fn allowed(&self) -> bool {
        self.blocker.is_none()
    }

    fn decision(&self) -> &'static str {
        if self.allowed() { "allowed" } else { "blocked" }
    }
}

pub(crate) fn run_agent_preflight(
    project_root: &Path,
    task: &str,
    allow_wide_scope: bool,
    json: bool,
    check: bool,
) -> anyhow::Result<()> {
    let preflight = build_agent_preflight(project_root, task, allow_wide_scope);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render_json(&preflight))?
        );
    } else {
        print!("{}", render_text(&preflight));
    }
    if check && !preflight.allowed() {
        anyhow::bail!("agent preflight blocked: {}", preflight.reason);
    }
    Ok(())
}

pub(crate) fn prepare_agent_start_scope(
    project_root: &Path,
    task: &str,
    allow_wide_scope: bool,
) -> anyhow::Result<crate::task_scope::ResolvedTaskScope> {
    let preflight = build_agent_preflight(project_root, task, allow_wide_scope);
    if !preflight.allowed() {
        print!("{}", render_text(&preflight));
        anyhow::bail!("agent preflight blocked: {}", preflight.reason);
    }
    Ok(crate::task_scope::resolve_task_scope(project_root, task))
}

pub(crate) fn blocked_agent_start(
    project_root: &Path,
    task: &str,
    allow_wide_scope: bool,
) -> Option<AgentPreflight> {
    let preflight = build_agent_preflight(project_root, task, allow_wide_scope);
    (!preflight.allowed()).then_some(preflight)
}

pub(crate) fn build_agent_preflight(
    project_root: &Path,
    task: &str,
    allow_wide_scope: bool,
) -> AgentPreflight {
    let scope = crate::task_scope::resolve_task_scope(project_root, task);
    let blocker = crate::scope_guard::wide_scope_blocker(
        project_root,
        task,
        scope.narrowed,
        allow_wide_scope,
    )
    .map(str::to_string);
    let reason = decision_reason(scope.narrowed, allow_wide_scope, blocker.as_deref());
    let policy_source = policy_source(task, scope.narrowed, allow_wide_scope, blocker.as_deref());
    let suggested_scopes = if blocker.is_some() {
        suggested_child_scopes(project_root)
    } else {
        Vec::new()
    };
    AgentPreflight {
        task: task_preview(task),
        launch: scope.launch_label,
        active_exists: scope.active_root.exists(),
        active: scope.active_label,
        narrowed: scope.narrowed,
        allow_wide_scope,
        policy_source,
        suggested_scopes,
        blocker,
        reason,
    }
}

pub(crate) fn render_text(preflight: &AgentPreflight) -> String {
    let scope = if preflight.narrowed {
        "NARROWED"
    } else {
        "WIDE"
    };
    let active_exists = if preflight.active_exists { "yes" } else { "no" };
    let allow_wide = if preflight.allow_wide_scope {
        "yes"
    } else {
        "no"
    };
    let suggested = if preflight.suggested_scopes.is_empty() {
        "(none)".into()
    } else {
        preflight.suggested_scopes.join(", ")
    };
    format!(
        "Agent preflight:\n  Task: {}\n  Launch: {}\n  Active: {}\n  Scope: {}\n  Active exists: {}\n  Allow wide policy: {}\n  Policy source: {}\n  Suggested child scopes: {}\n  Decision: {}\n  Reason: {}\n",
        preflight.task,
        preflight.launch,
        preflight.active,
        scope,
        active_exists,
        allow_wide,
        preflight.policy_source,
        suggested,
        preflight.decision().to_uppercase(),
        preflight.reason,
    )
}

fn render_json(preflight: &AgentPreflight) -> serde_json::Value {
    serde_json::json!({
        "task": preflight.task,
        "launch": preflight.launch,
        "active": preflight.active,
        "narrowed": preflight.narrowed,
        "active_exists": preflight.active_exists,
        "allow_wide_scope": preflight.allow_wide_scope,
        "policy_source": preflight.policy_source,
        "suggested_scopes": preflight.suggested_scopes,
        "allowed": preflight.allowed(),
        "decision": preflight.decision(),
        "reason": preflight.reason,
        "blocker": preflight.blocker,
    })
}

fn suggested_child_scopes(project_root: &Path) -> Vec<String> {
    let mut scopes = std::fs::read_dir(project_root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().to_str().map(|name| format!("{name}/")))
                .filter(|name| !skip_suggested_scope(name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    scopes.sort();
    scopes.truncate(8);
    scopes
}

fn skip_suggested_scope(name: &str) -> bool {
    matches!(name, ".git/" | ".dsx/" | "target/" | "node_modules/") || name.starts_with('.')
}

fn policy_source(
    task: &str,
    narrowed: bool,
    allow_wide_scope: bool,
    blocker: Option<&str>,
) -> String {
    if blocker.is_some() {
        "container_guard".into()
    } else if narrowed {
        "task_scope".into()
    } else if allow_wide_scope {
        "allow_wide_policy".into()
    } else if crate::scope_guard::has_explicit_wide_intent(task) {
        "task_wide_intent".into()
    } else {
        "workspace_safe".into()
    }
}

fn decision_reason(narrowed: bool, allow_wide_scope: bool, blocker: Option<&str>) -> String {
    if let Some(blocker) = blocker {
        blocker.to_string()
    } else if narrowed {
        "Task selected a child scope; agent tools will be locked there.".into()
    } else if allow_wide_scope {
        "Wide launch scope is allowed by explicit CLI/config policy.".into()
    } else {
        "Launch scope is safe for a wide run or task explicitly requested wide scope.".into()
    }
}

fn task_preview(task: &str) -> String {
    const MAX_CHARS: usize = 240;
    let cleaned = dsx_agent::brief::clean_task_input(task);
    let mut preview: String = cleaned.chars().take(MAX_CHARS).collect();
    if cleaned.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }
    preview
}
