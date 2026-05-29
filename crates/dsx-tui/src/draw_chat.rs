//! Chat transcript rendering.

use crate::App;
use crate::i18n::tr;
use crate::types::Language;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub fn draw_chat(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            let (role_label, role_style) = role_label(self.lang, msg.role.as_str());
            if !lines.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(vec![Span::styled(role_label, role_style)]));

            for content_line in msg.content.lines() {
                if content_line.is_empty() {
                    lines.push(Line::from(""));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(content_line.to_string(), message_style(&msg.role)),
                    ]));
                }
            }
        }

        let total_lines = lines.len();
        let height = area.height.saturating_sub(2) as usize;
        let max_scroll = total_lines.saturating_sub(height);
        let current_scroll = if self.scroll_offset == 0 && max_scroll > 0 {
            max_scroll
        } else {
            (self.scroll_offset as usize).min(max_scroll)
        };
        let end = (current_scroll + height).min(total_lines);
        let visible_lines = if current_scroll < total_lines {
            lines[current_scroll..end].to_vec()
        } else {
            lines
        };

        let paragraph = Paragraph::new(Text::from(visible_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(Color::Green))
                    .title(Span::styled(
                        tr(self.lang, "chat_title"),
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

fn role_label(lang: Language, role: &str) -> (&'static str, Style) {
    match role {
        "user" => (
            match lang {
                Language::Russian => "▸ Пользователь",
                Language::Kazakh => "▸ Пайдаланушы",
                Language::Chinese => "▸ 用户",
                Language::English => "▸ User",
            },
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        "assistant" => (
            match lang {
                Language::Russian => "▸ Ассистент",
                Language::Kazakh => "▸ Ассистент",
                Language::Chinese => "▸ 助手",
                Language::English => "▸ Assistant",
            },
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        "system" => (
            match lang {
                Language::Russian => "⚙ Система",
                Language::Kazakh => "⚙ Жүйе",
                Language::Chinese => "⚙ 系统",
                Language::English => "⚙ System",
            },
            Style::default().fg(Color::DarkGray),
        ),
        "error" => (
            match lang {
                Language::Russian => "⚠ Ошибка",
                Language::Kazakh => "⚠ Қате",
                Language::Chinese => "⚠ 错误",
                Language::English => "⚠ Error",
            },
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        "tool" => (
            match lang {
                Language::Russian => "🛠 Вызов инструмента",
                Language::Kazakh => "🛠 Құрал шақыру",
                Language::Chinese => "🛠 工具执行",
                Language::English => "🛠 Tool Execution",
            },
            Style::default().fg(Color::LightMagenta),
        ),
        _ => ("▸ Message", Style::default().fg(Color::LightCyan)),
    }
}

fn message_style(role: &str) -> Style {
    match role {
        "assistant" => Style::default().fg(Color::Green),
        "user" => Style::default().fg(Color::LightCyan),
        "tool" => Style::default().fg(Color::LightCyan),
        _ => Style::default().fg(Color::DarkGray),
    }
}
