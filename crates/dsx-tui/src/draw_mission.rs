//! Mission Control view for live task state, scope, and tool activity.

use crate::App;
use crate::draw_budget::run_budget_line;
use crate::draw_mission_state::{
    empty_as, mission_tool_counts, scope_color, scope_guard_text, scope_status, task_state,
    tool_status_color,
};
use crate::draw_run_ledger::append_run_ledger_lines;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub fn draw_mission(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(10)])
            .split(area);
        let body = Layout::default()
            .direction(if rows[1].width >= 118 {
                Direction::Horizontal
            } else {
                Direction::Vertical
            })
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(rows[1]);

        frame.render_widget(summary_panel(self), rows[0]);
        frame.render_widget(task_panel(self), body[0]);
        frame.render_widget(ops_panel(self), body[1]);
    }
}

fn summary_panel(app: &App) -> Paragraph<'static> {
    let (ok, failed, blocked) = mission_tool_counts(app);
    Paragraph::new(Text::from(vec![
        Line::from(vec![
            label("Run "),
            value(task_state(app)),
            gap(),
            label("Scope "),
            value(scope_status(app)),
            gap(),
            label("Tools "),
            value(format!("ok {ok} / failed {failed} / blocked {blocked}")),
        ]),
        Line::from(vec![
            label("Budget "),
            value(empty_as(&app.budget_status, "not measured")),
            gap(),
            label("Run "),
            value(run_budget_line(app)),
        ]),
        Line::from(vec![
            label("Compact "),
            value(format!(
                "{} event(s), {} msg, ~{} tok saved",
                app.compaction_events, app.compacted_messages, app.estimated_tokens_saved
            )),
        ]),
    ]))
    .block(block(" Mission Control ", Color::LightCyan))
    .wrap(Wrap { trim: false })
}

fn task_panel(app: &App) -> Paragraph<'static> {
    let b = &app.task_brief;
    let mut lines = Vec::new();
    push_section(&mut lines, "Goal", &b.goal, Color::LightCyan, 4);
    push_section(&mut lines, "Done", &b.done, Color::LightGreen, 4);
    push_section(&mut lines, "Plan", &b.plan, Color::LightCyan, 6);
    push_section(&mut lines, "Last", &b.last_changes, Color::LightGreen, 4);
    push_section(&mut lines, "Next", &b.next_step, Color::LightMagenta, 3);
    push_section(
        &mut lines,
        "Architecture",
        &b.architecture,
        Color::LightBlue,
        4,
    );
    Paragraph::new(Text::from(lines))
        .block(block(" Goal / Done / Plan ", Color::Cyan))
        .wrap(Wrap { trim: false })
}

fn ops_panel(app: &App) -> Paragraph<'static> {
    let mut lines = Vec::new();
    push_inline(
        &mut lines,
        "Launch",
        &app.scope_lock.launch_scope,
        Color::DarkGray,
    );
    push_inline(
        &mut lines,
        "Active",
        &app.scope_lock.active_scope,
        Color::LightCyan,
    );
    push_inline(
        &mut lines,
        "Status",
        &app.scope_lock.status,
        scope_color(app),
    );
    push_inline(&mut lines, "Reason", &app.scope_lock.reason, Color::LightCyan);
    if !app.scope_lock.warning.trim().is_empty() {
        push_inline(
            &mut lines,
            "Warning",
            &app.scope_lock.warning,
            Color::LightGreen,
        );
    }
    push_inline(
        &mut lines,
        "Guard",
        &scope_guard_text(app),
        scope_color(app),
    );
    lines.push(Line::from(""));
    append_run_ledger_lines(app, &mut lines, 4);
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Recent tools",
        Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD),
    )]));
    if app.tool_timeline.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No tool calls in this task yet.",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for entry in app.tool_timeline.iter().rev().take(8).rev() {
            let color = tool_status_color(&entry.status);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<7}", entry.status),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(entry.name.clone(), Style::default().fg(Color::LightMagenta)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(entry.summary.clone(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    Paragraph::new(Text::from(lines))
        .block(block(" Scope / Tools ", Color::LightGreen))
        .wrap(Wrap { trim: false })
}

fn push_section(
    lines: &mut Vec<Line<'static>>,
    title: &'static str,
    value: &str,
    color: Color,
    limit: usize,
) {
    lines.push(Line::from(vec![Span::styled(
        format!("{title}:"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));
    for line in value.lines().take(limit) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::LightCyan)),
        ]));
    }
}

fn push_inline(lines: &mut Vec<Line<'static>>, title: &'static str, value: &str, color: Color) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("{title}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(empty_as(value, "none"), Style::default().fg(Color::LightCyan)),
    ]));
}

fn block(title: &'static str, color: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
}

fn label(text: &'static str) -> Span<'static> {
    Span::styled(
        text,
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    )
}

fn value(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(Color::LightCyan))
}

fn gap() -> Span<'static> {
    Span::raw("  ")
}
