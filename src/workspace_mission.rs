//! Compact workspace mission handoff across notes, runs, and safety checks.

use std::path::Path;

pub(crate) use crate::workspace_mission_output::mission_json;

pub(crate) struct MissionSnapshot {
    pub(crate) workspace: String,
    pub(crate) goal: String,
    pub(crate) done: String,
    pub(crate) plan: String,
    pub(crate) last_changes: String,
    pub(crate) next_step: String,
    pub(crate) active_scope: String,
    pub(crate) architecture: String,
    pub(crate) run_health: MissionRunHealth,
    pub(crate) line_limit: MissionLineLimit,
    pub(crate) notes: Vec<MissionNote>,
}

#[derive(Default)]
pub(crate) struct MissionRunHealth {
    pub(crate) recent_runs: usize,
    pub(crate) running_runs: usize,
    pub(crate) failed_runs: usize,
    pub(crate) cancelled_runs: usize,
    pub(crate) total_tokens: i64,
    pub(crate) scope_violations: i64,
}

pub(crate) struct MissionLineLimit {
    pub(crate) ok: bool,
    pub(crate) violations: Vec<String>,
    pub(crate) pressure: Vec<String>,
}

pub(crate) struct MissionNote {
    pub(crate) scope: String,
    pub(crate) saved: bool,
    pub(crate) next_step: String,
    pub(crate) architecture: String,
}

pub async fn run_workspace_mission(project_root: &Path, limit: u32, all: bool, json: bool) {
    match collect_mission_snapshot(project_root, limit, all).await {
        Ok(snapshot) if json => println!("{}", mission_json(&snapshot)),
        Ok(snapshot) => crate::workspace_mission_output::print_mission(&snapshot, all),
        Err(e) => println!("Workspace mission error: {e}"),
    }
}

pub(crate) async fn collect_mission_snapshot(
    project_root: &Path,
    limit: u32,
    all: bool,
) -> anyhow::Result<MissionSnapshot> {
    let notes = crate::workspace_notes::collect_workspace_notes(project_root, limit, all).await?;
    let runs = crate::workspace_runs::collect_agent_runs(project_root, limit, all).await?;
    let primary = notes
        .iter()
        .find(|note| note.saved)
        .or_else(|| notes.first());
    let run_health = mission_run_health(&runs);
    let line_limit = mission_line_limit(project_root)?;

    Ok(MissionSnapshot {
        workspace: project_root.display().to_string(),
        goal: note_field(primary, |note| &note.goal, "No saved goal yet."),
        done: note_field(primary, |note| &note.done, "No saved completion state yet."),
        plan: note_field(primary, |note| &note.plan, "No saved plan yet."),
        last_changes: note_field(primary, |note| &note.last_changes, "No saved changes yet."),
        next_step: note_field(primary, |note| &note.next_step, "No saved next step yet."),
        active_scope: active_scope(project_root, primary, &runs),
        architecture: note_field(
            primary,
            |note| &note.architecture,
            "No architecture note yet.",
        ),
        run_health,
        line_limit,
        notes: notes.into_iter().map(mission_note).collect(),
    })
}

fn mission_run_health(runs: &[crate::workspace_runs::LocatedRun]) -> MissionRunHealth {
    let mut health = MissionRunHealth {
        recent_runs: runs.len(),
        ..Default::default()
    };
    for located in runs {
        let run = &located.run;
        if run.status == "running" {
            health.running_runs += 1;
        }
        if run.status == "failed" || run.error.is_some() {
            health.failed_runs += 1;
        }
        if run.cancelled {
            health.cancelled_runs += 1;
        }
        health.total_tokens += run.total_tokens;
        health.scope_violations += run.scope_violations;
    }
    health
}

fn mission_line_limit(project_root: &Path) -> anyhow::Result<MissionLineLimit> {
    let violations =
        crate::line_limit::rust_line_violations(project_root, crate::line_limit::MAX_RS_LINES)?
            .into_iter()
            .map(line_label)
            .collect::<Vec<_>>();
    let pressure = crate::line_limit::rust_line_pressure(
        project_root,
        crate::line_limit::PRESSURE_RS_LINES,
        crate::line_limit::MAX_RS_LINES,
    )?
    .into_iter()
    .map(line_label)
    .collect::<Vec<_>>();
    Ok(MissionLineLimit {
        ok: violations.is_empty(),
        violations,
        pressure,
    })
}

fn mission_note(note: crate::workspace_notes::WorkspaceNote) -> MissionNote {
    MissionNote {
        scope: note.label,
        saved: note.saved,
        next_step: note.next_step,
        architecture: note.architecture,
    }
}

fn active_scope(
    project_root: &Path,
    primary: Option<&crate::workspace_notes::WorkspaceNote>,
    runs: &[crate::workspace_runs::LocatedRun],
) -> String {
    runs.first()
        .map(|located| located.run.active_scope.trim())
        .filter(|scope| !scope.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| primary.map(|note| scope_to_path(project_root, &note.label)))
        .unwrap_or_else(|| project_root.display().to_string())
}

fn scope_to_path(project_root: &Path, label: &str) -> String {
    if label == "." {
        project_root.display().to_string()
    } else {
        project_root.join(label).display().to_string()
    }
}

fn note_field(
    note: Option<&crate::workspace_notes::WorkspaceNote>,
    pick: impl Fn(&crate::workspace_notes::WorkspaceNote) -> &String,
    fallback: &'static str,
) -> String {
    note.map(pick)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .into()
}

fn line_label(item: crate::line_limit::FileLineCount) -> String {
    format!("{}={} lines", item.path.display(), item.lines)
}

#[cfg(test)]
#[path = "workspace_mission_tests.rs"]
mod tests;
