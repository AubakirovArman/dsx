//! DSX TUI — rendering and drawing layout helpers.

use crate::types::Language;
use crate::App;
use crate::i18n::tr;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

impl App {
    pub fn draw(&self, frame: &mut Frame) {
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .split(frame.area());

        let top = main[0];

        let mut horizontal_constraints = Vec::new();
        if self.show_file_tree && !self.file_tree.is_empty() {
            horizontal_constraints.push(Constraint::Percentage(20));
        }
        horizontal_constraints.push(Constraint::Min(40));
        if !self.current_reasoning.is_empty() {
            horizontal_constraints.push(Constraint::Percentage(33));
        }

        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(horizontal_constraints)
            .split(top);

        let mut pane_idx = 0;

        if self.show_file_tree && !self.file_tree.is_empty() {
            self.draw_file_tree(frame, panes[pane_idx]);
            pane_idx += 1;
        }

        let main_workspace_area = panes[pane_idx];
        pane_idx += 1;

        if !self.current_reasoning.is_empty() {
            self.draw_reasoning(frame, panes[pane_idx]);
        }

        if self.show_diff {
            self.draw_diff(frame, main_workspace_area);
        } else if self.show_settings {
            self.draw_settings(frame, main_workspace_area);
        } else {
            self.draw_chat(frame, main_workspace_area);
        }

        self.draw_input(frame, main[1]);
        self.draw_status(frame, main[2]);
    }

    fn draw_file_tree(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.file_tree.iter().map(|f| {
            let style = if f.ends_with('/') || f.contains('/') {
                Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)
            } else if f.ends_with(".rs") {
                Style::default().fg(Color::Yellow)
            } else if f.ends_with(".toml") || f.ends_with(".md") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(format!("  {}", f), style)))
        }).collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(tr(self.lang, "sidebar_title"), Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)))
            )
            .style(Style::default());

        frame.render_widget(list, area);
    }

    fn draw_reasoning(&self, frame: &mut Frame, area: Rect) {
        let text = Paragraph::new(self.current_reasoning.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::LightMagenta))
                    .title(Span::styled(tr(self.lang, "reasoning_title"), Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)))
            )
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
            .wrap(Wrap { trim: false });

        frame.render_widget(text, area);
    }

    fn draw_diff(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(tr(self.lang, "diff_banner"), Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(tr(self.lang, "diff_header_desc")));
        lines.push(Line::from("   ─────────────────────────────────────────────────────────────────────────────"));
        lines.push(Line::from(""));

        if self.current_diff.trim().is_empty() {
            lines.push(Line::from(tr(self.lang, "diff_clean")));
        } else {
            for diff_line in self.current_diff.lines() {
                let style = if diff_line.starts_with('+') && !diff_line.starts_with("+++") {
                    Style::default().fg(Color::Green)
                } else if diff_line.starts_with('-') && !diff_line.starts_with("---") {
                    Style::default().fg(Color::Red)
                } else if diff_line.starts_with("@@") {
                    Style::default().fg(Color::Cyan)
                } else if diff_line.starts_with("diff") || diff_line.starts_with("index") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(diff_line, style),
                ]));
            }
        }

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(Span::styled(tr(self.lang, "diff_title"), Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)))
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
