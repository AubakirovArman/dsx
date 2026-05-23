//! Settings screen rendering.

use crate::App;
use crate::i18n::tr;
use crate::types::Language;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
};

impl App {
    pub fn draw_settings(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = vec![
            Line::from(""),
            banner(tr(self.lang, "settings_header_banner")),
            Line::from(""),
            Line::from(tr(self.lang, "settings_header_desc")),
            separator(),
            Line::from(""),
        ];

        self.push_setting(
            &mut lines,
            0,
            tr(self.lang, "settings_opt_security"),
            &self.mode,
        );
        self.push_setting(
            &mut lines,
            1,
            tr(self.lang, "settings_opt_model"),
            &self.model,
        );
        self.push_setting(
            &mut lines,
            2,
            tr(self.lang, "settings_opt_sidebar"),
            self.tree_label(),
        );
        self.push_setting(
            &mut lines,
            3,
            tr(self.lang, "settings_opt_language"),
            self.lang.display_name(),
        );
        self.push_setting(&mut lines, 4, self.api_base_label(), &self.api_base);
        self.push_setting(&mut lines, 5, self.api_key_label(), &self.masked_key());
        self.push_action(&mut lines, 6, tr(self.lang, "settings_opt_clear"));

        lines.push(separator());
        lines.push(Line::from(tr(self.lang, "telemetry_title")));
        lines.push(Line::from(tr(self.lang, "telemetry_db")));
        lines.push(metric_line(
            tr(self.lang, "telemetry_cost"),
            format!("${:.4}", self.cost),
            Color::LightGreen,
        ));
        lines.push(metric_line(
            tr(self.lang, "telemetry_tokens"),
            self.tokens.to_string(),
            Color::White,
        ));

        let paragraph = Paragraph::new(Text::from(lines)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(
                    tr(self.lang, "settings_title"),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )),
        );

        frame.render_widget(paragraph, area);
    }

    fn push_setting(&self, lines: &mut Vec<Line<'_>>, idx: usize, label: &str, value: &str) {
        let selected = self.settings_cursor == idx;
        let prefix = if selected { "  ▸ " } else { "    " };
        let style = selected_style(selected);
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Cyan)),
            Span::styled(format!("{label}: "), style),
            chip(value.to_string(), Color::White),
            Span::styled(
                format!("   {}", setting_hint(self.lang, idx)),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        lines.push(Line::from(""));
    }

    fn push_action(&self, lines: &mut Vec<Line<'_>>, idx: usize, label: &str) {
        let selected = self.settings_cursor == idx;
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "  ▸ " } else { "    " },
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(label.to_string(), selected_style(selected)),
            chip(
                tr(self.lang, "settings_clear_action").to_string(),
                Color::LightRed,
            ),
        ]));
        lines.push(Line::from(""));
    }

    fn tree_label(&self) -> &'static str {
        match (self.show_file_tree, self.lang) {
            (true, Language::Russian) => "ПОКАЗАТЬ",
            (false, Language::Russian) => "СКРЫТЬ",
            (true, Language::Kazakh) => "КӨРСЕТУ",
            (false, Language::Kazakh) => "ЖАСЫРУ",
            (true, Language::Chinese) => "显示",
            (false, Language::Chinese) => "隐藏",
            (true, Language::English) => "SHOW",
            (false, Language::English) => "HIDE",
        }
    }

    fn api_base_label(&self) -> &'static str {
        match self.lang {
            Language::Russian => "АДРЕС API ENDPOINT BASE",
            Language::Kazakh => "API ENDPOINT НЕГІЗГІ МЕКЕНІ",
            Language::Chinese => "API 接口服务基准地址",
            Language::English => "API ENDPOINT BASE URL",
        }
    }

    fn api_key_label(&self) -> &'static str {
        match self.lang {
            Language::Russian => "КЛЮЧ АВТОРИЗАЦИИ API KEY",
            Language::Kazakh => "API АВТОРИЗАЦИЯ КЛЮЧІ",
            Language::Chinese => "API 授权验证密钥",
            Language::English => "API AUTHORIZATION KEY",
        }
    }

    fn masked_key(&self) -> String {
        if self.api_key.is_empty() {
            return match self.lang {
                Language::Russian => "⚠ КЛЮЧ НЕ НАЙДЕН",
                Language::Kazakh => "⚠ КЛЮЧ ТАБЫЛМАДЫ",
                Language::Chinese => "⚠ 未检测到密钥",
                Language::English => "⚠ KEY NOT DETECTED",
            }
            .into();
        }
        if self.api_key.len() > 10 {
            format!(
                "{}...{}",
                &self.api_key[..5],
                &self.api_key[self.api_key.len() - 4..]
            )
        } else {
            "✓ Loaded".into()
        }
    }
}

fn setting_hint(lang: Language, idx: usize) -> &'static str {
    match (lang, idx) {
        (Language::Russian, 0) => "(Спросить / Авто / Yolo / План)",
        (Language::Russian, 1) => "(v4-pro Думающая / v4-flash Быстрая)",
        (Language::Russian, 2) => "(Панель файлов)",
        (Language::Russian, 3) => "(Язык интерфейса)",
        (Language::Russian, 4) => "(Официальный API / локальные endpoints)",
        (Language::Russian, 5) => "(DEEPSEEK_API_KEY)",
        (Language::Kazakh, 0) => "(Сұрау / Авто / Yolo / Жоспар)",
        (Language::Kazakh, 1) => "(v4-pro Ойлайтын / v4-flash Жылдам)",
        (Language::Kazakh, 2) => "(Файлдар панелі)",
        (Language::Kazakh, 3) => "(Интерфейс тілі)",
        (Language::Chinese, 0) => "(提示确认 / 自动运行 / YOLO / 仅规划)",
        (Language::Chinese, 1) => "(v4-pro 深度思考 / v4-flash 高速响应)",
        (Language::Chinese, 2) => "(文件侧边栏)",
        (Language::Chinese, 3) => "(界面语言)",
        (Language::English, 0) => "(Ask / Auto / Yolo / Plan)",
        (Language::English, 1) => "(v4-pro Thinking / v4-flash Fast)",
        (Language::English, 2) => "(File tree panel)",
        (Language::English, 3) => "(Interface language)",
        (_, 4) => "(API endpoint)",
        (_, 5) => "(DEEPSEEK_API_KEY)",
        _ => "",
    }
}

fn banner(label: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled(
            label,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn separator() -> Line<'static> {
    Line::from("   ─────────────────────────────────────────────────────────────────────────────")
}

fn selected_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn chip(value: String, bg: Color) -> Span<'static> {
    Span::styled(
        format!("[ {value} ]"),
        Style::default()
            .fg(Color::Black)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    )
}

fn metric_line(label: &'static str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::raw(label),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}
