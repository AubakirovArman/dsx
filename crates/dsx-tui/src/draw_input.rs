//! Input and approval prompt rendering.

use crate::App;
use crate::i18n::tr;
use crate::types::{AgentTask, Language};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

impl App {
    pub fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let widget = if let Some(ref approval) = self.pending_approval {
            let body = approval_body(self.lang, &approval.tool_name, &approval.arguments);
            Paragraph::new(body)
                .block(input_block(
                    tr(self.lang, "input_auth_title"),
                    Style::default().fg(Color::LightRed),
                ))
                .style(
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )
                .wrap(Wrap { trim: false })
        } else {
            let (prompt, title, style) = input_state(self);
            Paragraph::new(prompt)
                .block(input_block(title, style))
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: false })
        };

        frame.render_widget(widget, area);
    }
}

fn approval_body(lang: Language, tool: &str, args: &str) -> String {
    let template = match lang {
        Language::Russian => {
            " ⚠️  ТРЕБУЕТСЯ ПОДТВЕРЖДЕНИЕ БЕЗОПАСНОСТИ:\n  Инструмент: [{}] хочет запуститься в вашей рабочей области.\n  Аргументы: {}\n  ▸ Нажмите [Y] для ОДОБРЕНИЯ  |  [N] для ОТКЛОНЕНИЯ"
        }
        Language::Kazakh => {
            " ⚠️  ҚАУІПСІЗДІК АВТОРИЗАЦИЯСЫ ТАЛАП ЕТІЛЕДІ:\n  Құрал: [{}] жұмыс аймағында іске қосылғысы келеді.\n  Аргументтер: {}\n  ▸ [Y] рұқсат  |  [N] бас тарту"
        }
        Language::Chinese => {
            " ⚠️  需要安全授权验证:\n  工具: [{}] 申请在您的工作区运行。\n  参数: {}\n  ▸ 按 [Y] 同意  |  按 [N] 拒绝"
        }
        Language::English => {
            " ⚠️  SECURITY AUTHORIZATION REQUIRED:\n  Tool: [{}] wants to run in your workspace.\n  Arguments: {}\n  ▸ Press [Y] to APPROVE  |  [N] to DENY"
        }
    };
    template.replacen("{}", tool, 1).replacen("{}", args, 1)
}

fn input_state(app: &App) -> (String, &'static str, Style) {
    match &app.agent_task {
        AgentTask::Idle => (
            format!("  {}█", app.input),
            tr(app.lang, "input_title_idle"),
            Style::default().fg(Color::LightCyan),
        ),
        AgentTask::Running(desc) => (
            format!("  ⟳ {}...", running_label(app.lang, desc)),
            tr(app.lang, "input_title_running"),
            Style::default().fg(Color::LightYellow),
        ),
        AgentTask::Done(summary) => (
            format!("  ✓ {}  |  {}█", summary, app.input),
            tr(app.lang, "input_title_done"),
            Style::default().fg(Color::LightGreen),
        ),
        AgentTask::Error(err) => (
            format!("  ✗ {}  |  {}█", err, app.input),
            tr(app.lang, "input_title_error"),
            Style::default().fg(Color::LightRed),
        ),
    }
}

fn running_label(lang: Language, desc: &str) -> String {
    match lang {
        Language::Russian => format!("Выполнение: {desc}"),
        Language::Kazakh => format!("Орындау: {desc}"),
        Language::Chinese => format!("处理中: {desc}"),
        Language::English => format!("Processing stream: {desc}"),
    }
}

fn input_block(title: &'static str, style: Style) -> Block<'static> {
    Block::default()
        .style(Style::default().bg(Color::Black))
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(style)
        .title(Span::styled(title, style.add_modifier(Modifier::BOLD)))
}
