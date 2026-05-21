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
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
            .split(area);
        self.draw_task_brief_panel(frame, split[0]);
        self.draw_tool_timeline_panel(frame, split[1]);
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

    fn draw_tool_timeline_panel(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();
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
