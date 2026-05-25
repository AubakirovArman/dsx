//! Dedicated tool timeline view.

use crate::App;
use crate::draw_run_ledger::append_run_ledger_lines;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub fn draw_tools(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line<'static>> = vec![
            Line::from(vec![
                Span::styled("Active scope: ", Style::default().fg(Color::LightCyan)),
                Span::styled(
                    scope_text(&self.scope_lock.active_scope).to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Scope guard: ", Style::default().fg(scope_color(self))),
                Span::styled(scope_guard_text(self), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Tool totals: ", Style::default().fg(Color::LightYellow)),
                Span::styled(tool_totals_text(self), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
        ];
        append_run_ledger_lines(self, &mut lines, 6);
        lines.push(Line::from(""));
        if self.compaction_events > 0 {
            lines.push(compaction_line(self));
            lines.push(Line::from(""));
        }
        if self.tool_timeline.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No tool calls in this task yet.",
                Style::default().fg(Color::DarkGray),
            )]));
        } else {
            append_tools(self, &mut lines);
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .style(Style::default().bg(Color::Black))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(Span::styled(
                        " Tools / Scope Guard ",
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

fn append_tools(app: &App, lines: &mut Vec<Line<'static>>) {
    for (idx, entry) in app.tool_timeline.iter().rev().take(20).rev().enumerate() {
        let color = status_color(&entry.status);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:02} ", idx + 1),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{:<7}", entry.status),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(entry.name.clone(), Style::default().fg(Color::LightYellow)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(entry.summary.clone(), Style::default().fg(Color::Gray)),
        ]));
    }
}

fn compaction_line(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::styled("compact ", Style::default().fg(Color::LightCyan)),
        Span::styled(
            format!(
                "{} event(s), {} msg, ~{} tok saved",
                app.compaction_events, app.compacted_messages, app.estimated_tokens_saved
            ),
            Style::default().fg(Color::Gray),
        ),
    ])
}

fn status_color(status: &str) -> Color {
    match status {
        "ok" => Color::LightGreen,
        "blocked" => Color::LightRed,
        "failed" => Color::LightRed,
        _ => Color::White,
    }
}

fn scope_color(app: &App) -> Color {
    if app.scope_violations > 0 {
        Color::LightRed
    } else {
        Color::LightGreen
    }
}

fn scope_guard_text(app: &App) -> String {
    if app.scope_violations == 0 {
        return "0 blocked escape(s)".into();
    }
    format!(
        "{} blocked escape(s); last: {}",
        app.scope_violations, app.last_scope_violation
    )
}

fn tool_totals_text(app: &App) -> String {
    let (ok, failed, blocked) = tool_counts(app);
    format!("ok {ok} / failed {failed} / blocked {blocked}")
}

fn tool_counts(app: &App) -> (usize, usize, usize) {
    let ok = app
        .tool_timeline
        .iter()
        .filter(|entry| entry.status == "ok")
        .count();
    let failed = app
        .tool_timeline
        .iter()
        .filter(|entry| entry.status == "failed")
        .count();
    let blocked = app
        .tool_timeline
        .iter()
        .filter(|entry| entry.status == "blocked")
        .count();
    (ok, failed, blocked)
}

fn scope_text(scope: &str) -> &str {
    if scope.trim().is_empty() {
        "(none)"
    } else {
        scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolTimelineEntry;

    #[test]
    fn tool_totals_count_visible_statuses() {
        let mut app = App::new();
        for status in ["ok", "failed", "blocked", "ok"] {
            app.tool_timeline.push(ToolTimelineEntry {
                name: "tool".into(),
                status: status.into(),
                summary: "summary".into(),
            });
        }

        assert_eq!(tool_counts(&app), (2, 1, 1));
        assert_eq!(tool_totals_text(&app), "ok 2 / failed 1 / blocked 1");
    }
}
