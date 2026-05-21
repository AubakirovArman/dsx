//! DSX TUI — rendering and drawing layout helpers.

use crate::App;
use crate::i18n::tr;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

impl App {
    pub fn draw(&self, frame: &mut Frame) {
        let input_height = if self.pending_approval.is_some() {
            6
        } else {
            3
        };

        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(input_height),
                Constraint::Length(1),
            ])
            .split(frame.area());

        let top = main[0];

        let show_reasoning = !self.current_reasoning.is_empty()
            || (self.model == "v4-pro"
                && matches!(self.agent_task, crate::types::AgentTask::Running(_)));

        let mut horizontal_constraints = Vec::new();
        if self.show_file_tree && !self.file_tree.is_empty() {
            horizontal_constraints.push(Constraint::Percentage(20));
        }
        horizontal_constraints.push(Constraint::Min(40));
        if show_reasoning {
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

        if show_reasoning {
            self.draw_reasoning(frame, panes[pane_idx]);
        }

        if self.show_diff {
            self.draw_diff(frame, main_workspace_area);
        } else if self.show_settings {
            self.draw_settings(frame, main_workspace_area);
        } else {
            self.draw_workspace(frame, main_workspace_area);
        }

        self.draw_input(frame, main[1]);
        self.draw_status(frame, main[2]);
    }

    fn draw_file_tree(&self, frame: &mut Frame, area: Rect) {
        let height = area.height.saturating_sub(3) as usize; // reserve space for title/borders and "+N more" indicator
        let show_more = self.file_tree.len() > height;

        let visible_files: Vec<&String> = if show_more {
            self.file_tree.iter().take(height).collect()
        } else {
            self.file_tree.iter().collect()
        };

        let mut items: Vec<ListItem> = visible_files
            .iter()
            .map(|f| {
                let style = if f.ends_with('/') || f.contains('/') {
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD)
                } else if f.ends_with(".rs") {
                    Style::default().fg(Color::Yellow)
                } else if f.ends_with(".toml") || f.ends_with(".md") {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(format!("  {}", f), style)))
            })
            .collect();

        if show_more {
            let remaining = self.file_tree.len() - height;
            items.push(ListItem::new(Line::from(Span::styled(
                format!("  ... (+{} more files)", remaining),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ))));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(
                        tr(self.lang, "sidebar_title"),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .style(Style::default());

        frame.render_widget(list, area);
    }

    fn draw_reasoning(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for line in self.current_reasoning.lines() {
            lines.push(Line::from(vec![Span::styled(
                line,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        if lines.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                match self.lang {
                    crate::types::Language::Russian => "  ⟳ Соединение установлено...",
                    crate::types::Language::Kazakh => "  ⟳ Байланыс орнатылды...",
                    crate::types::Language::Chinese => "  ⟳ API 握手成功...",
                    crate::types::Language::English => "  ⟳ Handshake established...",
                },
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![Span::styled(
                match self.lang {
                    crate::types::Language::Russian => "  ⌛ Обработка промпта в очереди...",
                    crate::types::Language::Kazakh => "  ⌛ Кезекті өңдеу жүріп жатыр...",
                    crate::types::Language::Chinese => "  ⌛ 正在排队并处理提示词上下文...",
                    crate::types::Language::English => "  ⌛ Processing prompt & queuing...",
                },
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]));
            lines.push(Line::from(vec![Span::styled(
                match self.lang {
                    crate::types::Language::Russian => "  ⚡ Пожалуйста, подождите (v4-pro)...",
                    crate::types::Language::Kazakh => "  ⚡ Күте тұрыңыз (v4-pro)...",
                    crate::types::Language::Chinese => "  ⚡ 请稍后 (v4-pro)...",
                    crate::types::Language::English => "  ⚡ Please wait (v4-pro)...",
                },
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        // Auto-scroll reasoning process: keep thoughts locked to the bottom
        let total_lines = lines.len();
        let height = area.height.saturating_sub(2) as usize;
        let max_scroll = total_lines.saturating_sub(height);

        let visible_lines = if max_scroll > 0 {
            lines[max_scroll..].to_vec()
        } else {
            lines
        };

        let text = Text::from(visible_lines);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::LightMagenta))
                    .title(Span::styled(
                        tr(self.lang, "reasoning_title"),
                        Style::default()
                            .fg(Color::LightMagenta)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn draw_diff(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("   "),
                Span::styled(
                    tr(self.lang, "diff_banner"),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(tr(self.lang, "diff_header_desc")),
            Line::from(
                "   ─────────────────────────────────────────────────────────────────────────────",
            ),
            Line::from(""),
        ];

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
                    .title(Span::styled(
                        tr(self.lang, "diff_title"),
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
