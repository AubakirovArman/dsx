//! Shared run-ledger lines for Mission Control and Tools views.

use crate::{App, RunLedgerItem};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(crate) fn run_ledger_summary_text(app: &App) -> String {
    let runs = &app.run_ledger;
    format!(
        "runs {} / running {} / failed {} / stale {} / tok {} / blocked {}",
        runs.total, runs.running, runs.failed, runs.stale, runs.total_tokens, runs.scope_violations
    )
}

pub(crate) fn append_run_ledger_lines(
    app: &App,
    lines: &mut Vec<Line<'static>>,
    recent_limit: usize,
) {
    lines.push(Line::from(vec![Span::styled(
        "Run ledger",
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            run_ledger_summary_text(app),
            Style::default().fg(Color::White),
        ),
    ]));
    if app.run_ledger.estimated_tokens_saved > 0 {
        lines.push(Line::from(vec![
            Span::raw("  compact saved "),
            Span::styled(
                format!("~{} tok", app.run_ledger.estimated_tokens_saved),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }
    append_recent_runs(&app.run_ledger.recent, lines, recent_limit);
}

fn append_recent_runs(
    recent: &[RunLedgerItem],
    lines: &mut Vec<Line<'static>>,
    recent_limit: usize,
) {
    if recent.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No saved agent runs yet.",
            Style::default().fg(Color::Cyan),
        )]));
        return;
    }
    for run in recent.iter().take(recent_limit) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:<8}", run.status),
                Style::default()
                    .fg(status_color(&run.status))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(run.scope.clone(), Style::default().fg(Color::LightYellow)),
            Span::raw(format!(" {} tok", run.total_tokens)),
            Span::raw(format!(" scope:{}", run.scope_violations)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{} {}", run.id, truncate(&run.task, 82)),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }
}

fn status_color(status: &str) -> Color {
    match status {
        "completed" => Color::LightGreen,
        "running" => Color::LightYellow,
        "failed" | "cancelled" => Color::LightRed,
        _ => Color::White,
    }
}

fn truncate(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        text.push_str("...");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_ledger_summary_tracks_health_counts() {
        let mut app = App::new();
        app.run_ledger.total = 3;
        app.run_ledger.running = 1;
        app.run_ledger.failed = 1;
        app.run_ledger.stale = 2;
        app.run_ledger.total_tokens = 42;
        app.run_ledger.scope_violations = 4;

        assert_eq!(
            run_ledger_summary_text(&app),
            "runs 3 / running 1 / failed 1 / stale 2 / tok 42 / blocked 4"
        );
    }
}
