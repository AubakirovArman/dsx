//! Mapping workspace run audit data into TUI run-ledger panels.

pub(crate) fn panel_from_audit(
    audit: &crate::workspace_audit::WorkspaceAudit,
) -> dsx_tui::RunLedgerPanel {
    let recent = audit
        .runs
        .iter()
        .take(6)
        .map(|run| dsx_tui::RunLedgerItem {
            id: run.id.clone(),
            status: run.status.clone(),
            scope: run.scope.clone(),
            task: run.task.clone(),
            total_tokens: run.total_tokens,
            scope_violations: run.scope_violations,
        })
        .collect::<Vec<_>>();
    dsx_tui::RunLedgerPanel {
        total: audit.runs.len(),
        running: audit.running_runs,
        failed: audit
            .runs
            .iter()
            .filter(|run| run.status == "failed")
            .count(),
        cancelled: audit
            .runs
            .iter()
            .filter(|run| run.status == "cancelled")
            .count(),
        stale: audit.stale_runs,
        total_tokens: audit.runs.iter().map(|run| run.total_tokens).sum(),
        estimated_tokens_saved: audit
            .runs
            .iter()
            .map(|run| run.estimated_tokens_saved)
            .sum(),
        scope_violations: audit.scope_violations,
        recent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_from_audit_summarizes_recent_run_health() {
        let audit = crate::workspace_audit::WorkspaceAudit {
            workspace: "/tmp/project".into(),
            budget: "ok".into(),
            running_runs: 1,
            stale_runs: 2,
            line_violations: Vec::new(),
            line_pressure: Vec::new(),
            runs: vec![crate::workspace_audit::AuditRun {
                id: "12345678".into(),
                status: "failed".into(),
                scope: "1234".into(),
                contract: "launch -> active".into(),
                task: "build".into(),
                total_tokens: 99,
                estimated_cost_usd: 0.01,
                compaction_events: 1,
                estimated_tokens_saved: 300,
                scope_violations: 3,
            }],
            notes: Vec::new(),
            scope_violations: 3,
        };

        let panel = panel_from_audit(&audit);

        assert_eq!(panel.total, 1);
        assert_eq!(panel.running, 1);
        assert_eq!(panel.failed, 1);
        assert_eq!(panel.stale, 2);
        assert_eq!(panel.total_tokens, 99);
        assert_eq!(panel.estimated_tokens_saved, 300);
        assert_eq!(panel.recent[0].scope, "1234");
    }
}
