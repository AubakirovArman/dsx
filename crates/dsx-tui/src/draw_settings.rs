//! DSX TUI — rendering for settings, chat, input prompt, and status bar.

use crate::types::{Language, AgentTask};
use crate::App;
use crate::i18n::tr;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

impl App {
    pub fn draw_settings(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(tr(self.lang, "settings_header_banner"), Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(tr(self.lang, "settings_header_desc")));
        lines.push(Line::from("   ─────────────────────────────────────────────────────────────────────────────"));
        lines.push(Line::from(""));

        // Option 1: Permission Mode
        let is_sel0 = self.settings_cursor == 0;
        let prefix0 = if is_sel0 { "  ▸ " } else { "    " };
        let style0 = if is_sel0 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        lines.push(Line::from(vec![
            Span::styled(prefix0, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(tr(self.lang, "settings_opt_security"), style0),
            Span::styled(format!("[ {} ]", self.mode.to_uppercase()), Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(match self.lang {
                Language::Russian => "   (Политика: Спросить / Авто / Yolo / План)",
                Language::Kazakh => "   (Саясат: Сұрау / Авто / Yolo / Жоспар)",
                Language::Chinese => "   (授权模式: 提示确认 / 自动运行 / YOLO免确认 / 仅作规划)",
                Language::English => "   (Approval Policy: Ask / Auto / Yolo / Plan)",
            }, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));

        // Option 2: Model Choice
        let is_sel1 = self.settings_cursor == 1;
        let prefix1 = if is_sel1 { "  ▸ " } else { "    " };
        let style1 = if is_sel1 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        lines.push(Line::from(vec![
            Span::styled(prefix1, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(tr(self.lang, "settings_opt_model"), style1),
            Span::styled(format!("[ {} ]", self.model.to_uppercase()), Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(match self.lang {
                Language::Russian => "   (v4-pro Думающая / v4-flash Быстрая)",
                Language::Kazakh => "   (v4-pro Ойлайтын / v4-flash Жылдам)",
                Language::Chinese => "   (v4-pro 深度思考 / v4-flash 高速响应)",
                Language::English => "   (v4-pro Thinking / v4-flash Non-Thinking)",
            }, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));

        // Option 3: Toggle Tree Explorer
        let is_sel2 = self.settings_cursor == 2;
        let prefix2 = if is_sel2 { "  ▸ " } else { "    " };
        let style2 = if is_sel2 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        let tree_status = if self.show_file_tree {
            match self.lang {
                Language::Russian => "ПОКАЗАТЬ",
                Language::Kazakh => "КӨРСЕТУ",
                Language::Chinese => "显示",
                Language::English => "SHOW",
            }
        } else {
            match self.lang {
                Language::Russian => "СКРЫТЬ",
                Language::Kazakh => "ЖАСЫРУ",
                Language::Chinese => "隐藏",
                Language::English => "HIDE",
            }
        };
        lines.push(Line::from(vec![
            Span::styled(prefix2, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(tr(self.lang, "settings_opt_sidebar"), style2),
            Span::styled(format!("[ {} ]", tree_status), Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(match self.lang {
                Language::Russian => "   (Показать боковую панель проводника файлов)",
                Language::Kazakh => "   (Бүйірлік файлдар панелін қосу)",
                Language::Chinese => "   (开启或关闭工作区文件资源管理器侧边栏)",
                Language::English => "   (Toggle workspace tree layout)",
            }, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));

        // Option 4: Language Selection
        let is_sel3 = self.settings_cursor == 3;
        let prefix3 = if is_sel3 { "  ▸ " } else { "    " };
        let style3 = if is_sel3 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        lines.push(Line::from(vec![
            Span::styled(prefix3, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(tr(self.lang, "settings_opt_language"), style3),
            Span::styled(format!("[ {} ]", self.lang.display_name()), Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(match self.lang {
                Language::Russian => "   (Смена языка интерфейса TUI)",
                Language::Kazakh => "   (TUI интерфейсі тілін ауыстыру)",
                Language::Chinese => "   (切换系统 TUI 界面显示语言)",
                Language::English => "   (Switch TUI interface language)",
            }, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));

        // Option 5: API Base URL
        let is_sel4 = self.settings_cursor == 4;
        let prefix4 = if is_sel4 { "  ▸ " } else { "    " };
        let style4 = if is_sel4 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        let api_base_label = match self.lang {
            Language::Russian => "АДРЕС API ENDPOINT BASE:   ",
            Language::Kazakh => "API ENDPOINT НЕГІЗГІ МЕКЕНІ: ",
            Language::Chinese => "API 接口服务基准地址:     ",
            Language::English => "API ENDPOINT BASE URL:     ",
        };
        lines.push(Line::from(vec![
            Span::styled(prefix4, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(api_base_label, style4),
            Span::styled(format!("[ {} ]", self.api_base), Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(match self.lang {
                Language::Russian => "   (Официальный API / Локальный Ollama / vLLM / OpenAI)",
                Language::Kazakh => "   (Ресми API / Жергілікті Ollama / vLLM / OpenAI)",
                Language::Chinese => "   (官方接口 / 本地 Ollama / vLLM / OpenAI 代理)",
                Language::English => "   (Official API / Local Ollama / vLLM / OpenAI proxy)",
            }, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));

        // Option 6: API Authorization Key
        let is_sel5 = self.settings_cursor == 5;
        let prefix5 = if is_sel5 { "  ▸ " } else { "    " };
        let style5 = if is_sel5 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        let key_label = match self.lang {
            Language::Russian => "КЛЮЧ АВТОРИЗАЦИИ API KEY:  ",
            Language::Kazakh => "API АВТОРИЗАЦИЯ КЛЮЧІ:      ",
            Language::Chinese => "API 授权验证密钥:          ",
            Language::English => "API AUTHORIZATION KEY:     ",
        };

        let masked_key = if self.api_key.is_empty() {
            match self.lang {
                Language::Russian => "⚠ КЛЮЧ НЕ НАЙДЕН".to_string(),
                Language::Kazakh => "⚠ КЛЮЧ ТАБЫЛМАДЫ".to_string(),
                Language::Chinese => "⚠ 未检测到密钥".to_string(),
                Language::English => "⚠ KEY NOT DETECTED".to_string(),
            }
        } else {
            if self.api_key.len() > 10 {
                format!("{}...{}", &self.api_key[..5], &self.api_key[self.api_key.len()-4..])
            } else {
                "✓ Loaded".to_string()
            }
        };

        lines.push(Line::from(vec![
            Span::styled(prefix5, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(key_label, style5),
            Span::styled(format!("[ {} ]", masked_key), Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(match self.lang {
                Language::Russian => "   (Считывается из переменной DEEPSEEK_API_KEY)",
                Language::Kazakh => "   (DEEPSEEK_API_KEY айнымалысынан оқылады)",
                Language::Chinese => "   (读取自系统环境变量 DEEPSEEK_API_KEY)",
                Language::English => "   (Loaded from DEEPSEEK_API_KEY env var)",
            }, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));

        // Option 7: Clear Chat History
        let is_sel6 = self.settings_cursor == 6;
        let prefix6 = if is_sel6 { "  ▸ " } else { "    " };
        let style6 = if is_sel6 { Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        lines.push(Line::from(vec![
            Span::styled(prefix6, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(tr(self.lang, "settings_opt_clear"), style6),
            Span::styled(tr(self.lang, "settings_clear_action"), Style::default().fg(Color::White).bg(Color::LightRed).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));

        // Metadata System Information
        lines.push(Line::from("   ─────────────────────────────────────────────────────────────────────────────"));
        lines.push(Line::from(tr(self.lang, "telemetry_title")));
        lines.push(Line::from(tr(self.lang, "telemetry_db")));
        lines.push(Line::from(vec![
            Span::raw(tr(self.lang, "telemetry_cost")),
            Span::styled(format!("${:.4}", self.cost), Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::raw(tr(self.lang, "telemetry_tokens")),
            Span::styled(self.tokens.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));

        let paragraph = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled(tr(self.lang, "settings_title"), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)))
            );

        frame.render_widget(paragraph, area);
    }

    pub fn draw_chat(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            let (role_label, role_style) = match msg.role.as_str() {
                "user" => (
                    match self.lang {
                        Language::Russian => "▸ Пользователь",
                        Language::Kazakh => "▸ Пайдаланушы",
                        Language::Chinese => "▸ 用户",
                        Language::English => "▸ User",
                    },
                    Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD),
                ),
                "assistant" => (
                    match self.lang {
                        Language::Russian => "▸ Ассистент",
                        Language::Kazakh => "▸ Ассистент",
                        Language::Chinese => "▸ 助手",
                        Language::English => "▸ Assistant",
                    },
                    Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
                ),
                "system" => (
                    match self.lang {
                        Language::Russian => "⚙ Система",
                        Language::Kazakh => "⚙ Жүйе",
                        Language::Chinese => "⚙ 系统",
                        Language::English => "⚙ System",
                    },
                    Style::default().fg(Color::DarkGray),
                ),
                "error" => (
                    match self.lang {
                        Language::Russian => "⚠ Ошибка",
                        Language::Kazakh => "⚠ Қате",
                        Language::Chinese => "⚠ 错误",
                        Language::English => "⚠ Error",
                    },
                    Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
                ),
                "tool" => (
                    match self.lang {
                        Language::Russian => "🛠 Вызов инструмента",
                        Language::Kazakh => "🛠 Құрал шақыру",
                        Language::Chinese => "🛠 工具执行",
                        Language::English => "🛠 Tool Execution",
                    },
                    Style::default().fg(Color::LightYellow),
                ),
                _ => (
                    "▸ Message",
                    Style::default().fg(Color::White),
                ),
            };

            if !lines.is_empty() {
                lines.push(Line::from(""));
            }

            lines.push(Line::from(vec![
                Span::styled(role_label, role_style),
            ]));

            for content_line in msg.content.lines() {
                if content_line.is_empty() {
                    lines.push(Line::from(""));
                } else {
                    let text_style = match msg.role.as_str() {
                        "assistant" => Style::default().fg(Color::Green),
                        "user" => Style::default().fg(Color::LightCyan),
                        "tool" => Style::default().fg(Color::White),
                        _ => Style::default().fg(Color::Gray),
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(content_line, text_style),
                    ]));
                }
            }
        }

        let total_lines = lines.len();
        let height = area.height.saturating_sub(2) as usize; // subtract border heights
        
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

        let text = Text::from(visible_lines);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Green))
                    .title(Span::styled(tr(self.lang, "chat_title"), Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)))
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    pub fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let input_widget = if let Some(ref approval) = self.pending_approval {
            let format_body = match self.lang {
                Language::Russian => " ⚠️  ТРЕБУЕТСЯ ПОДТВЕРЖДЕНИЕ БЕЗОПАСНОСТИ:\n  Инструмент: [{}] хочет запуститься в вашей рабочей области.\n  Аргументы: {}\n  ▸ Нажмите [Y] для ОДОБРЕНИЯ (Разрешить)  |  [N] для ОТКЛОНЕНИЯ (Заблокировать)",
                Language::Kazakh => " ⚠️  ҚАУІПСІЗДІК АВТОРИЗАЦИЯСЫ ТАЛАП ЕТІЛЕДІ:\n  Құрал: [{}] жұмыс аймағында іске қосылғысы келеді.\n  Аргументтер: {}\n  ▸ Рұқсат беру үшін [Y] басыңыз  |  Бас тарту үшін [N] басыңыз",
                Language::Chinese => " ⚠️  需要安全授权验证:\n  工具: [{}] 申请在您的工作区运行。\n  参数: {}\n  ▸ 按 [Y] 同意授权 (允许)  |  按 [N] 拒绝请求 (拒绝)",
                Language::English => " ⚠️  SECURITY AUTHORIZATION REQUIRED:\n  Tool: [{}] wants to run in your workspace.\n  Arguments: {}\n  ▸ Press [Y] to APPROVE (Allow)  |  [N] to DENY (Reject)",
            };
            let alert_text = format_body
                .replacen("{}", &approval.tool_name, 1)
                .replacen("{}", &approval.arguments, 1);

            Paragraph::new(alert_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::LightRed))
                        .title(Span::styled(tr(self.lang, "input_auth_title"), Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)))
                )
                .style(Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: false })
        } else {
            let (prompt, block_title, style) = match &self.agent_task {
                AgentTask::Idle => (
                    format!("  {}█", self.input),
                    tr(self.lang, "input_title_idle"),
                    Style::default().fg(Color::LightCyan),
                ),
                AgentTask::Running(desc) => {
                    let desc_loc = match self.lang {
                        Language::Russian => format!("Выполнение: {}...", desc),
                        Language::Kazakh => format!("Орындау: {}...", desc),
                        Language::Chinese => format!("处理中: {}...", desc),
                        Language::English => format!("Processing stream: {}...", desc),
                    };
                    (
                        format!("  ⟳ {}", desc_loc),
                        tr(self.lang, "input_title_running"),
                        Style::default().fg(Color::LightYellow),
                    )
                }
                AgentTask::Done(summary) => (
                    format!("  ✓ {}  |  {}█", summary, self.input),
                    tr(self.lang, "input_title_done"),
                    Style::default().fg(Color::LightGreen),
                ),
                AgentTask::Error(err) => (
                    format!("  ✗ {}  |  {}█", err, self.input),
                    tr(self.lang, "input_title_error"),
                    Style::default().fg(Color::LightRed),
                ),
            };

            Paragraph::new(prompt)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(style)
                        .title(Span::styled(block_title, style.add_modifier(Modifier::BOLD)))
                )
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: false })
        };

        frame.render_widget(input_widget, area);
    }

    pub fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let task_indicator = match &self.agent_task {
            AgentTask::Idle => match self.lang {
                Language::Russian => "⏸ ОЖИДАНИЕ",
                Language::Kazakh => "⏸ КҮТУ",
                Language::Chinese => "⏸ 空闲",
                Language::English => "⏸ IDLE",
            },
            AgentTask::Running(_) => match self.lang {
                Language::Russian => "⚡ МЫШЛЕНИЕ",
                Language::Kazakh => "⚡ ОЙЛАУ",
                Language::Chinese => "⚡ 推理中",
                Language::English => "⚡ THINKING",
            },
            AgentTask::Done(_) => match self.lang {
                Language::Russian => "✓ УСПЕХ",
                Language::Kazakh => "✓ СӘТТІ",
                Language::Chinese => "✓ 成功",
                Language::English => "✓ SUCCESS",
            },
            AgentTask::Error(_) => match self.lang {
                Language::Russian => "✗ ПРЕРВАНО",
                Language::Kazakh => "✗ ҮЗІЛДІ",
                Language::Chinese => "✗ 中断",
                Language::English => "✗ INTERRUPTED",
            },
        };

        let mode_color = match self.mode.as_str() {
            "yolo" => Color::LightRed,
            "auto" => Color::LightYellow,
            "ask" => Color::LightCyan,
            "plan-only" => Color::LightBlue,
            _ => Color::DarkGray,
        };

        let cost_str = if self.cost > 0.0 {
            format!("${:.4}", self.cost)
        } else {
            "$0".into()
        };

        let spans = if self.show_diff {
            vec![
                Span::styled(format!(" {} ", task_indicator), Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(format!(" [{}] ", self.mode.to_uppercase()), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled("Ctrl+D", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(tr(self.lang, "status_diff_toggle")),
                Span::styled("Ctrl+C", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                Span::raw(tr(self.lang, "status_quit")),
            ]
        } else if self.show_settings {
            vec![
                Span::styled(format!(" {} ", task_indicator), Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(format!(" [{}] ", self.mode.to_uppercase()), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled("↑/↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(match self.lang {
                    Language::Russian => ":нав ",
                    Language::Kazakh => ":бағ ",
                    Language::Chinese => ":选 ",
                    Language::English => ":nav ",
                }),
                Span::styled("←/→", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(match self.lang {
                    Language::Russian => ":изм ",
                    Language::Kazakh => ":өзг ",
                    Language::Chinese => ":改 ",
                    Language::English => ":mod ",
                }),
                Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" Esc", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                Span::raw(match self.lang {
                    Language::Russian => ":вых ",
                    Language::Kazakh => ":қайту ",
                    Language::Chinese => ":返 ",
                    Language::English => ":exit ",
                }),
                Span::styled("Ctrl+C", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                Span::raw(tr(self.lang, "status_quit")),
            ]
        } else {
            let width = area.width;
            if width >= 110 {
                // Full gorgeous layout
                vec![
                    Span::styled(format!(" {} ", task_indicator), Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(format!(" [{}] ", self.mode.to_uppercase()), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" | model: "),
                    Span::styled(self.model.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw(" | tokens: "),
                    Span::styled(self.tokens.to_string(), Style::default().fg(Color::White)),
                    Span::raw(" | cost: "),
                    Span::styled(cost_str, Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                    Span::raw(" | "),
                    Span::styled("Ctrl+S", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_settings_toggle")),
                    Span::styled("Ctrl+T", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_tree_toggle")),
                    Span::styled("Ctrl+D", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_diff_toggle")),
                    Span::styled("Ctrl+U", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_undo_toggle")),
                    Span::styled("Ctrl+C", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_quit")),
                ]
            } else if width >= 80 {
                // Compact responsive layout
                vec![
                    Span::styled(format!(" {} ", task_indicator), Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(format!(" [{}] ", self.mode.to_uppercase()), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" | cost: "),
                    Span::styled(cost_str, Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                    Span::raw(" | "),
                    Span::styled("Ctrl+S", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_settings_toggle")),
                    Span::styled("Ctrl+D", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_diff_toggle")),
                    Span::styled("Ctrl+U", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_undo_toggle")),
                ]
            } else {
                // Minimal responsive layout for narrow screens
                vec![
                    Span::styled(format!(" {} ", task_indicator), Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(format!(" [{}] ", self.mode.to_uppercase()), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" | "),
                    Span::styled("Ctrl+S", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(tr(self.lang, "status_settings_toggle")),
                    Span::styled("Ctrl+C", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                ]
            }
        };

        let status_bar = Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::Black).fg(Color::Gray));

        frame.render_widget(status_bar, area);
    }
}
