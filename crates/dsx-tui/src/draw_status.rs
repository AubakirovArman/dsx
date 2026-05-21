//! Bottom status bar rendering.

use crate::App;
use crate::i18n::tr;
use crate::types::{AgentTask, Language};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::path::Path;

impl App {
    pub fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let task = task_indicator(self.lang, &self.agent_task);
        let mode_color = match self.mode.as_str() {
            "yolo" => Color::LightRed,
            "auto" => Color::LightYellow,
            "ask" => Color::LightCyan,
            "plan-only" => Color::LightBlue,
            _ => Color::DarkGray,
        };
        let running = matches!(self.agent_task, AgentTask::Running(_));
        let cost = if self.cost > 0.0 {
            format!("${:.4}", self.cost)
        } else {
            "$0".into()
        };

        let mut spans = vec![
            Span::styled(
                format!(" {task} "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" [{}] ", self.mode.to_uppercase()),
                Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
            ),
        ];

        if self.show_diff {
            spans.extend([
                plain(" | "),
                key("Ctrl+D"),
                plain(tr(self.lang, "status_diff_toggle")),
            ]);
        } else if self.show_settings {
            spans.extend(settings_keys(self.lang));
        } else if self.show_tools {
            spans.extend(tools_keys(self.lang));
        } else if area.width >= 110 {
            spans.extend([
                plain(" | model: "),
                strong_owned(self.model.clone()),
                plain(" | tokens: "),
                plain_owned(self.tokens.to_string()),
                plain(" | cost: "),
                strong_owned(cost),
                plain(" | scope: "),
                strong_owned(scope_badge(self)),
                plain(" | fuse: "),
                plain_owned(self.budget_status.clone()),
                plain(" | "),
            ]);
            spans.extend(main_keys(self.lang, true, running));
        } else if area.width >= 80 {
            spans.extend([
                plain(" | cost: "),
                strong_owned(cost),
                plain(" | scope: "),
                strong_owned(scope_badge(self)),
                plain(" | "),
            ]);
            spans.extend(main_keys(self.lang, false, running));
        } else {
            spans.extend([
                plain(" | "),
                key("Ctrl+S"),
                plain(tr(self.lang, "status_settings_toggle")),
            ]);
        }

        spans.extend([key("Ctrl+C"), plain(tr(self.lang, "status_quit"))]);
        let status_bar = Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::Black).fg(Color::Gray));
        frame.render_widget(status_bar, area);
    }
}

fn scope_badge(app: &App) -> String {
    let active = app.scope_lock.active_scope.trim();
    if active.is_empty() {
        return "none".into();
    }
    let label = Path::new(active)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(active);
    match app.scope_lock.status.as_str() {
        "Narrowed" => format!("narrow:{label}"),
        "Blocked" => format!("blocked:{label}"),
        _ => format!("wide:{label}"),
    }
}

fn task_indicator(lang: Language, task: &AgentTask) -> &'static str {
    match task {
        AgentTask::Idle => match lang {
            Language::Russian => "⏸ ОЖИДАНИЕ",
            Language::Kazakh => "⏸ КҮТУ",
            Language::Chinese => "⏸ 空闲",
            Language::English => "⏸ IDLE",
        },
        AgentTask::Running(_) => match lang {
            Language::Russian => "⚡ МЫШЛЕНИЕ",
            Language::Kazakh => "⚡ ОЙЛАУ",
            Language::Chinese => "⚡ 推理中",
            Language::English => "⚡ THINKING",
        },
        AgentTask::Done(_) => match lang {
            Language::Russian => "✓ УСПЕХ",
            Language::Kazakh => "✓ СӘТТІ",
            Language::Chinese => "✓ 成功",
            Language::English => "✓ SUCCESS",
        },
        AgentTask::Error(_) => match lang {
            Language::Russian => "✗ ПРЕРВАНО",
            Language::Kazakh => "✗ ҮЗІЛДІ",
            Language::Chinese => "✗ 中断",
            Language::English => "✗ INTERRUPTED",
        },
    }
}

fn settings_keys(lang: Language) -> Vec<Span<'static>> {
    vec![
        plain(" | "),
        key("↑/↓"),
        plain(match lang {
            Language::Russian => ":нав ",
            Language::Kazakh => ":бағ ",
            Language::Chinese => ":选 ",
            Language::English => ":nav ",
        }),
        key("←/→"),
        plain(match lang {
            Language::Russian => ":изм ",
            Language::Kazakh => ":өзг ",
            Language::Chinese => ":改 ",
            Language::English => ":mod ",
        }),
        key("Enter"),
        plain(" "),
        key("Esc"),
    ]
}

fn tools_keys(lang: Language) -> Vec<Span<'static>> {
    vec![
        plain(" | "),
        key("Esc"),
        plain(match lang {
            Language::Russian => ":закрыть ",
            Language::Kazakh => ":жабу ",
            Language::Chinese => ":关闭 ",
            Language::English => ":close ",
        }),
        key("Ctrl+L"),
        plain(":tools "),
    ]
}

fn main_keys(lang: Language, include_tree: bool, include_stop: bool) -> Vec<Span<'static>> {
    let mut spans = vec![key("Ctrl+S"), plain(tr(lang, "status_settings_toggle"))];
    if include_tree {
        spans.extend([key("Ctrl+T"), plain(tr(lang, "status_tree_toggle"))]);
    }
    if include_stop {
        spans.extend([key("Ctrl+K"), plain(tr(lang, "status_stop_toggle"))]);
    }
    spans.extend([
        key("Ctrl+D"),
        plain(tr(lang, "status_diff_toggle")),
        key("Ctrl+L"),
        plain(":tools "),
        key("Ctrl+U"),
        plain(tr(lang, "status_undo_toggle")),
    ]);
    spans
}

fn key(label: &'static str) -> Span<'static> {
    Span::styled(
        label,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

fn strong_owned(label: String) -> Span<'static> {
    Span::styled(
        label,
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
    )
}

fn plain(label: &'static str) -> Span<'static> {
    Span::raw(label)
}

fn plain_owned(label: String) -> Span<'static> {
    Span::raw(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_badge_shows_narrow_active_folder() {
        let mut app = App::new();
        app.begin_task_scoped("build", "/tmp/sites", "/tmp/sites/1234", true);

        assert_eq!(scope_badge(&app), "narrow:1234");
    }

    #[test]
    fn scope_badge_marks_blocked_scope() {
        let mut app = App::new();
        app.scope_lock.active_scope = "/tmp/sites".into();
        app.scope_lock.status = "Blocked".into();

        assert_eq!(scope_badge(&app), "blocked:sites");
    }
}
