//! Secondary panes for file tree, reasoning stream, and diff view.

use crate::App;
use crate::i18n::tr;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

impl App {
    pub(crate) fn draw_file_tree(&self, frame: &mut Frame, area: Rect) {
        let height = area.height.saturating_sub(3) as usize;
        let show_more = self.file_tree.len() > height;
        let visible_files: Vec<&String> = if show_more {
            self.file_tree.iter().take(height).collect()
        } else {
            self.file_tree.iter().collect()
        };

        let mut items: Vec<ListItem> = visible_files
            .iter()
            .map(|f| ListItem::new(Line::from(Span::styled(format!("  {}", f), file_style(f)))))
            .collect();

        if show_more {
            let remaining = self.file_tree.len() - height;
            items.push(ListItem::new(Line::from(Span::styled(
                format!("  ... (+{} more files)", remaining),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::ITALIC),
            ))));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(Color::Green))
                    .title(Span::styled(
                        tr(self.lang, "sidebar_title"),
                        Style::default()
                            .fg(Color::LightCyan)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .style(Style::default());

        frame.render_widget(list, area);
    }

    pub(crate) fn draw_reasoning(&self, frame: &mut Frame, area: Rect) {
        let mut lines = reasoning_lines(self);
        if lines.is_empty() {
            lines = reasoning_placeholder(self.lang);
        }

        let height = area.height.saturating_sub(2) as usize;
        let max_scroll = lines.len().saturating_sub(height);
        let visible_lines = if max_scroll > 0 {
            lines[max_scroll..].to_vec()
        } else {
            lines
        };

        let paragraph = Paragraph::new(Text::from(visible_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
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

    pub(crate) fn draw_diff(&self, frame: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(Text::from(diff_lines(self)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(Color::Green))
                    .title(Span::styled(
                        tr(self.lang, "diff_title"),
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

fn file_style(path: &str) -> Style {
    if path.ends_with('/') || path.contains('/') {
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD)
    } else if path.ends_with(".rs") {
        Style::default().fg(Color::Green)
    } else if path.ends_with(".toml") || path.ends_with(".md") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Cyan)
    }
}

fn reasoning_lines(app: &App) -> Vec<Line<'_>> {
    app.current_reasoning
        .lines()
        .map(|line| {
            Line::from(vec![Span::styled(
                line,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::ITALIC),
            )])
        })
        .collect()
}

fn reasoning_placeholder(lang: crate::types::Language) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        placeholder_line(
            match lang {
                crate::types::Language::Russian => "  ⟳ Соединение установлено...",
                crate::types::Language::Kazakh => "  ⟳ Байланыс орнатылды...",
                crate::types::Language::Chinese => "  ⟳ API 握手成功...",
                crate::types::Language::English => "  ⟳ Handshake established...",
            },
            Color::LightMagenta,
            true,
        ),
        placeholder_line(
            match lang {
                crate::types::Language::Russian => "  ⌛ Обработка промпта в очереди...",
                crate::types::Language::Kazakh => "  ⌛ Кезекті өңдеу жүріп жатыр...",
                crate::types::Language::Chinese => "  ⌛ 正在排队并处理提示词上下文...",
                crate::types::Language::English => "  ⌛ Processing prompt & queuing...",
            },
            Color::Green,
            false,
        ),
        placeholder_line(
            match lang {
                crate::types::Language::Russian => "  ⚡ Пожалуйста, подождите (v4-pro)...",
                crate::types::Language::Kazakh => "  ⚡ Күте тұрыңыз (v4-pro)...",
                crate::types::Language::Chinese => "  ⚡ 请稍后 (v4-pro)...",
                crate::types::Language::English => "  ⚡ Please wait (v4-pro)...",
            },
            Color::Green,
            false,
        ),
    ]
}

fn placeholder_line(text: &'static str, color: Color, bold: bool) -> Line<'static> {
    let mut style = Style::default().fg(color);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    } else {
        style = style.add_modifier(Modifier::ITALIC);
    }
    Line::from(vec![Span::styled(text, style)])
}

fn diff_lines(app: &App) -> Vec<Line<'_>> {
    let mut lines = diff_header(app.lang);
    if app.current_diff.trim().is_empty() {
        lines.push(Line::from(tr(app.lang, "diff_clean")));
    } else {
        for diff_line in app.current_diff.lines() {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(diff_line, diff_style(diff_line)),
            ]));
        }
    }
    lines
}

fn diff_header(lang: crate::types::Language) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                tr(lang, "diff_banner"),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(tr(lang, "diff_header_desc")),
        Line::from(
            "   ─────────────────────────────────────────────────────────────────────────────",
        ),
        Line::from(""),
    ]
}

fn diff_style(line: &str) -> Style {
    if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(Color::Green)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(Color::Red)
    } else if line.starts_with("@@") {
        Style::default().fg(Color::Cyan)
    } else if line.starts_with("diff") || line.starts_with("index") {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Cyan)
    }
}
