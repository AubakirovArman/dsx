//! Dedicated compact context capsule view.

use crate::{App, TaskBriefPanel};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub fn draw_context(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line<'static>> = Vec::new();
        append_brief(&self.task_brief, &mut lines);
        lines.push(Line::from(""));
        append_scope(self, &mut lines);
        lines.push(Line::from(""));
        append_folder_notes(self, &mut lines);
        lines.push(Line::from(""));
        append_compaction(self, &mut lines);

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::LightCyan))
                    .title(Span::styled(
                        " Context Capsule ",
                        Style::default()
                            .fg(Color::LightCyan)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

fn append_brief(brief: &TaskBriefPanel, lines: &mut Vec<Line<'static>>) {
    push_field(lines, "Goal", &brief.goal, Color::LightCyan, 3);
    push_field(lines, "Done", &brief.done, Color::LightGreen, 2);
    push_field(lines, "Plan", &brief.plan, Color::White, 5);
    push_field(lines, "Last", &brief.last_changes, Color::Yellow, 3);
    push_field(lines, "Next", &brief.next_step, Color::LightMagenta, 2);
    push_field(lines, "Constraints", &brief.constraints, Color::LightRed, 5);
    push_field(
        lines,
        "Architecture",
        &brief.architecture,
        Color::LightBlue,
        8,
    );
}

fn append_scope(app: &App, lines: &mut Vec<Line<'static>>) {
    push_inline(lines, "Launch", &app.scope_lock.launch_scope, Color::Gray);
    push_inline(
        lines,
        "Active",
        &app.scope_lock.active_scope,
        Color::LightCyan,
    );
    push_inline(lines, "Status", &app.scope_lock.status, scope_color(app));
    if !app.scope_lock.warning.trim().is_empty() {
        push_inline(lines, "Check", &app.scope_lock.warning, Color::LightYellow);
    }
}

fn append_folder_notes(app: &App, lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(vec![Span::styled(
        "Folder notes:",
        Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD),
    )]));
    if app.folder_notes.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  none loaded",
            Style::default().fg(Color::DarkGray),
        )]));
        return;
    }
    let focused = app.focused_folder_note_index().unwrap_or(0);
    let start = focused.saturating_sub(7);
    for (index, note) in app.folder_notes.iter().enumerate().skip(start).take(8) {
        let selected = index == focused;
        lines.push(Line::from(vec![
            Span::raw(if selected { "> " } else { "  " }),
            Span::styled(
                note.folder.clone(),
                Style::default()
                    .fg(if selected {
                        Color::LightCyan
                    } else {
                        Color::LightBlue
                    })
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::raw(" "),
            Span::styled(note.summary.clone(), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    next: "),
            Span::styled(note.next_step.clone(), Style::default().fg(Color::Gray)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    arch: "),
            Span::styled(
                note.architecture.clone(),
                Style::default().fg(Color::LightBlue),
            ),
        ]));
    }
    if let Some(note) = app.focused_folder_note() {
        lines.push(Line::from(""));
        push_inline(lines, "Focused", &note.folder, Color::LightCyan);
        if let Some(scope) = app.focused_folder_scope() {
            push_inline(lines, "Focused path", &scope, Color::Gray);
        }
        push_field(
            lines,
            "Focused next",
            &note.next_step,
            Color::LightMagenta,
            2,
        );
        push_field(
            lines,
            "Focused arch",
            &note.architecture,
            Color::LightBlue,
            3,
        );
    }
}

fn append_compaction(app: &App, lines: &mut Vec<Line<'static>>) {
    let status = format!(
        "{} event(s), {} msg, ~{} tok saved",
        app.compaction_events, app.compacted_messages, app.estimated_tokens_saved
    );
    push_inline(lines, "Compaction", &status, Color::LightCyan);
}

fn push_field(
    lines: &mut Vec<Line<'static>>,
    label: &'static str,
    value: &str,
    color: Color,
    limit: usize,
) {
    lines.push(Line::from(vec![Span::styled(
        format!("{label}:"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));
    for line in value.lines().take(limit) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::White)),
        ]));
    }
}

fn push_inline(lines: &mut Vec<Line<'static>>, label: &'static str, value: &str, color: Color) {
    let shown = if value.trim().is_empty() {
        "(none)"
    } else {
        value
    };
    lines.push(Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(shown.to_string(), Style::default().fg(Color::White)),
    ]));
}

fn scope_color(app: &App) -> Color {
    if app.scope_violations > 0 {
        Color::LightRed
    } else if app.scope_lock.status == "Narrowed" {
        Color::LightGreen
    } else {
        Color::LightYellow
    }
}
