//! Workflow panel rendering for compact plan state and tool timeline.

use crate::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub fn draw_workspace(&self, frame: &mut Frame, area: Rect) {
        if area.width < 82 || area.height < 14 {
            self.draw_chat(frame, area);
            return;
        }

        if area.width >= 116 {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(56), Constraint::Length(42)])
                .split(area);
            self.draw_chat(frame, panes[0]);
            self.draw_workflow_panel(frame, panes[1]);
        } else {
            let panes = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(8), Constraint::Length(10)])
                .split(area);
            self.draw_chat(frame, panes[0]);
            self.draw_workflow_panel(frame, panes[1]);
        }
    }

    fn draw_workflow_panel(&self, frame: &mut Frame, area: Rect) {
        if area.height >= 24 {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(34),
                    Constraint::Length(8),
                    Constraint::Length(7),
                    Constraint::Min(6),
                ])
                .split(area);
            self.draw_task_brief_panel(frame, split[0]);
            self.draw_scope_lock_panel(frame, split[1]);
            self.draw_folder_notes_panel(frame, split[2]);
            self.draw_tool_timeline_panel(frame, split[3]);
            return;
        }

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(44),
                Constraint::Length(8),
                Constraint::Min(6),
            ])
            .split(area);
        self.draw_task_brief_panel(frame, split[0]);
        self.draw_scope_lock_panel(frame, split[1]);
        self.draw_tool_timeline_panel(frame, split[2]);
    }

    fn draw_task_brief_panel(&self, frame: &mut Frame, area: Rect) {
        let b = &self.task_brief;
        let mut lines = Vec::new();
        push_field(&mut lines, "Goal", &b.goal, Color::LightCyan);
        push_field(&mut lines, "Done", &b.done, Color::LightGreen);
        push_field(&mut lines, "Plan", &b.plan, Color::White);
        push_field(&mut lines, "Last", &b.last_changes, Color::Yellow);
        push_field(&mut lines, "Next", &b.next_step, Color::LightMagenta);
        if !b.active_scope.trim().is_empty() {
            push_field(&mut lines, "Scope", &b.active_scope, Color::Gray);
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled(
                        " Plan / Done ",
                        Style::default()
                            .fg(Color::LightCyan)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn draw_folder_notes_panel(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();
        if self.folder_notes.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No folder summaries yet.",
                Style::default().fg(Color::DarkGray),
            )]));
        } else {
            for note in self.folder_notes.iter().take(4) {
                lines.push(Line::from(vec![
                    Span::styled(
                        note.folder.as_str(),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(note.summary.as_str(), Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  next: "),
                    Span::styled(note.next_step.as_str(), Style::default().fg(Color::Gray)),
                ]));
            }
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Blue))
                    .title(Span::styled(
                        " Folder Notes ",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn draw_tool_timeline_panel(&self, frame: &mut Frame, area: Rect) {
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
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        if self.tool_timeline.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "No tool calls in this task yet.",
                Style::default().fg(Color::DarkGray),
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
                    Span::styled(entry.name.as_str(), Style::default().fg(Color::LightYellow)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(entry.summary.as_str(), Style::default().fg(Color::Gray)),
                ]));
            }
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(Span::styled(
                        " Tools ",
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn draw_scope_lock_panel(&self, frame: &mut Frame, area: Rect) {
        let s = &self.scope_lock;
        let color = if self.scope_violations > 0 {
            Color::LightRed
        } else if s.status == "Narrowed" {
            Color::LightGreen
        } else {
            Color::LightYellow
        };
        let mut lines = Vec::new();
        push_inline(&mut lines, "Status", &s.status, color);
        push_inline(&mut lines, "Launch", &s.launch_scope, Color::Gray);
        push_inline(&mut lines, "Active", &s.active_scope, Color::LightCyan);
        push_inline(&mut lines, "Why", &s.reason, Color::White);
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
            push_inline(&mut lines, "Check", &s.warning, Color::LightYellow);
        }

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(color))
                    .title(Span::styled(
                        " Scope Lock ",
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

fn push_field(lines: &mut Vec<Line<'_>>, label: &'static str, value: &str, color: Color) {
    lines.push(Line::from(vec![Span::styled(
        format!("{label}:"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));
    for line in value.lines().take(4) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::White)),
        ]));
    }
}

fn push_inline(lines: &mut Vec<Line<'_>>, label: &'static str, value: &str, color: Color) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ]));
}
