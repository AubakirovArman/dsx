//! Workflow panel components for plan, scope, notes, and tool state.

use crate::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub(crate) fn draw_task_brief_panel(&self, frame: &mut Frame, area: Rect) {
        let b = &self.task_brief;
        let mut lines = Vec::new();
        push_field(&mut lines, "Goal", &b.goal, Color::LightCyan);
        push_field(&mut lines, "Done", &b.done, Color::LightGreen);
        push_field(&mut lines, "Plan", &b.plan, Color::Cyan);
        push_field(&mut lines, "Last", &b.last_changes, Color::Green);
        push_field(&mut lines, "Next", &b.next_step, Color::LightMagenta);
        if !b.active_scope.trim().is_empty() {
            push_field(&mut lines, "Scope", &b.active_scope, Color::Green);
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(panel_block(" Plan / Done ", Color::Cyan, Color::LightCyan))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    pub(crate) fn draw_folder_notes_panel(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();
        if self.folder_notes.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No folder summaries yet.",
                Style::default().fg(Color::Green),
            )]));
        } else {
            let focused = self.focused_folder_note_index().unwrap_or(0);
            let start = focused.saturating_sub(3);
            for (index, note) in self.folder_notes.iter().enumerate().skip(start).take(4) {
                let selected = index == focused;
                lines.push(Line::from(vec![
                    Span::raw(if selected { "> " } else { "  " }),
                    Span::styled(
                        note.folder.as_str(),
                        Style::default()
                            .fg(if selected {
                                Color::LightCyan
                            } else {
                                Color::LightCyan
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(note.summary.as_str(), Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  next: "),
                    Span::styled(note.next_step.as_str(), Style::default().fg(Color::Green)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  arch: "),
                    Span::styled(note.architecture.as_str(), Style::default().fg(Color::Cyan)),
                ]));
            }
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(panel_block(" Folder Notes ", Color::Cyan, Color::LightCyan))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    pub(crate) fn draw_tool_timeline_panel(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();
        if self.compaction_events > 0 {
            lines.push(Line::from(vec![
                Span::styled("compact ", Style::default().fg(Color::LightCyan)),
                Span::styled(
                    format!(
                        "{} event(s), {} msg, ~{} tok saved",
                        self.compaction_events,
                        self.compacted_messages,
                        self.estimated_tokens_saved
                    ),
                    Style::default().fg(Color::Green),
                ),
            ]));
        }
        if self.tool_timeline.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No tool calls in this task yet.",
                Style::default().fg(Color::Green),
            )]));
        } else {
            for entry in self.tool_timeline.iter().rev().take(8).rev() {
                let color = if entry.status == "ok" {
                    Color::LightGreen
                } else {
                    Color::LightRed
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", entry.status),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(entry.name.as_str(), Style::default().fg(Color::LightGreen)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(entry.summary.as_str(), Style::default().fg(Color::Green)),
                ]));
            }
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(panel_block(" Tools ", Color::Green, Color::LightGreen))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    pub(crate) fn draw_scope_lock_panel(&self, frame: &mut Frame, area: Rect) {
        let s = &self.scope_lock;
        let color = if self.scope_violations > 0 {
            Color::LightRed
        } else if s.status == "Narrowed" {
            Color::LightGreen
        } else {
            Color::LightGreen
        };
        let mut lines = Vec::new();
        push_inline(&mut lines, "Status", &s.status, color);
        push_inline(&mut lines, "Launch", &s.launch_scope, Color::Green);
        push_inline(&mut lines, "Active", &s.active_scope, Color::LightCyan);
        push_inline(&mut lines, "Why", &s.reason, Color::Cyan);
        if self.scope_violations > 0 {
            push_inline(
                &mut lines,
                "Blocked",
                &format!("{} scope escape(s)", self.scope_violations),
                Color::LightRed,
            );
            push_inline(
                &mut lines,
                "Last",
                &self.last_scope_violation,
                Color::LightRed,
            );
        }
        if !s.warning.trim().is_empty() {
            push_inline(&mut lines, "Check", &s.warning, Color::LightGreen);
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(panel_block(" Scope Lock ", color, color))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

fn panel_block<'a>(title: &'a str, border: Color, title_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            title,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
}

fn push_field(lines: &mut Vec<Line<'_>>, label: &'static str, value: &str, color: Color) {
    lines.push(Line::from(vec![Span::styled(
        format!("{label}:"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));
    for line in value.lines().take(4) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::Cyan)),
        ]));
    }
}

fn push_inline(lines: &mut Vec<Line<'_>>, label: &'static str, value: &str, color: Color) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::Cyan)),
    ]));
}
