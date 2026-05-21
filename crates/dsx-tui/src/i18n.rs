//! DSX TUI — i18n translation dictionary and lookup.

use crate::types::Language;

/// Retrieve localized text constant for a key and language.
pub fn tr(lang: Language, key: &str) -> &'static str {
    match lang {
        Language::English => match key {
            "welcome_msg" => {
                "DSX Code — DeepSeek V4 coding agent.\nType a task and press Enter. Ctrl+C to quit."
            }
            "sidebar_title" => " 📁 WORKSPACE ",
            "reasoning_title" => " 🧠 THOUGHT PROCESS ",
            "chat_title" => " 💻 COGNITIVE CORE ",
            "input_title_idle" => " ⌨️  TRANSMISSION INPUT ",
            "input_title_running" => " ⏳ COGNITIVE STREAM RUNNING ",
            "input_title_done" => " ⌨️  TRANSMISSION INPUT ",
            "input_title_error" => " ⚠️  TRANSMISSION INTERRUPTED ",
            "input_auth_title" => " 🔒 SECURE AUTHORIZATION GATEWAY ",
            "settings_title" => " ⚙️  SYSTEM SETTINGS CONFIGURATOR ",
            "settings_header_banner" => "  DSX CODE SYSTEM CONFIGURATOR  ",
            "settings_header_desc" => {
                "   Use [↑] / [↓] Arrow keys to navigate, [←] / [→] to modify values, and [Esc] to return."
            }
            "settings_opt_security" => "SECURITY PROTECTION MODE:  ",
            "settings_opt_model" => "PRIMARY REASONING MODEL:   ",
            "settings_opt_sidebar" => "WORKSPACE FILE SIDEBAR:    ",
            "settings_opt_language" => "SYSTEM INTERFACE LANGUAGE: ",
            "settings_opt_clear" => "CLEAR ACTIVE CHAT HISTORY: ",
            "settings_clear_action" => " [ PRESS ENTER TO WIPE CONVERSATION ] ",
            "telemetry_title" => "   CYBERNETIC TELEMETRY STATISTICS:",
            "telemetry_db" => {
                "    ▸ SQLite Persistence Schema:  Active and Bound (~/.dsx/sessions.db)"
            }
            "telemetry_cost" => "    ▸ Accumulated Session Cost:   ",
            "telemetry_tokens" => "    ▸ Total Counted Prompt Tokens: ",
            "diff_title" => " 🔍 ACTIVE WORKSPACE DIFFS ",
            "diff_banner" => "  DSX CODE INTERACTIVE WORKSPACE DIFF  ",
            "diff_header_desc" => "   Press [Ctrl+D] or [Esc] to exit the Diff Viewer.",
            "diff_clean" => {
                "   No modifications found in the active workspace. (Working tree clean)"
            }
            "status_m_toggle" => "m:mode ",
            "status_tree_toggle" => "Ctrl+T:files ",
            "status_settings_toggle" => "Ctrl+S:settings ",
            "status_stop_toggle" => "Ctrl+K:stop ",
            "status_diff_toggle" => "Ctrl+D:diff ",
            "status_undo_toggle" => "Ctrl+U:undo ",
            "status_quit" => "Ctrl+C:quit",
            _ => "",
        },
        Language::Russian => match key {
            "welcome_msg" => {
                "DSX Code — ИИ-агент для кодинга на базе DeepSeek V4.\nВведите задачу и нажмите Enter. Ctrl+C для выхода."
            }
            "sidebar_title" => " 📁 РАБОЧАЯ ОБЛАСТЬ ",
            "reasoning_title" => " 🧠 ХОД МЫСЛЕЙ ",
            "chat_title" => " 💻 КОГНИТИВНОЕ ЯДРО ",
            "input_title_idle" => " ⌨️  ВВОД ПЕРЕДАЧИ ",
            "input_title_running" => " ⏳ КОГНИТИВНЫЙ СТРИМ ЗАПУЩЕН ",
            "input_title_done" => " ⌨️  ВВОД ПЕРЕДАЧИ ",
            "input_title_error" => " ⚠️  ПЕРЕДАЧА ПРЕРВАНА ",
            "input_auth_title" => " 🔒 ШЛЮЗ БЕЗОПАСНОЙ АВТОРИЗАЦИИ ",
            "settings_title" => " ⚙️  СИСТЕМНЫЙ КОНФИГУРАТОР ",
            "settings_header_banner" => "  СИСТЕМНЫЙ КОНФИГУРАТОР DSX CODE  ",
            "settings_header_desc" => {
                "   Используйте стрелки [↑] / [↓] для навигации, [←] / [→] для изменения значений и [Esc] для возврата."
            }
            "settings_opt_security" => "РЕЖИМ ЗАЩИТЫ БЕЗОПАСНОСТИ: ",
            "settings_opt_model" => "ОСНОВНАЯ МОДЕЛЬ ИИ:        ",
            "settings_opt_sidebar" => "БОКОВАЯ ПАНЕЛЬ ФАЙЛОВ:     ",
            "settings_opt_language" => "ЯЗЫК ИНТЕРФЕЙСА СИСТЕМЫ:   ",
            "settings_opt_clear" => "ОЧИСТИТЬ АКТИВНЫЙ ДИАЛОГ:  ",
            "settings_clear_action" => " [ НАЖМИТЕ ENTER ДЛЯ ОЧИСТКИ ЧАТА ] ",
            "telemetry_title" => "   СТАТИСТИКА КИБЕРНЕТИЧЕСКОЙ ТЕЛЕМЕТРИИ:",
            "telemetry_db" => {
                "    ▸ SQLite Схема Персистентности: Активна и привязана (~/.dsx/sessions.db)"
            }
            "telemetry_cost" => "    ▸ Накопленная стоимость сессии: ",
            "telemetry_tokens" => "    ▸ Общее число токенов запроса: ",
            "diff_title" => " 🔍 АКТИВНЫЕ ИЗМЕНЕНИЯ (DIFF) ",
            "diff_banner" => "  ИНТЕРАКТИВНЫЙ ПРОСМОТР DIFF WORKSPACE  ",
            "diff_header_desc" => "   Нажмите [Ctrl+D] или [Esc], чтобы закрыть просмотрщик.",
            "diff_clean" => "   Изменений в рабочей области не обнаружено. (Репозиторий чист)",
            "status_m_toggle" => "m:режим ",
            "status_tree_toggle" => "Ctrl+T:файлы ",
            "status_settings_toggle" => "Ctrl+S:настройки ",
            "status_stop_toggle" => "Ctrl+K:стоп ",
            "status_diff_toggle" => "Ctrl+D:дифф ",
            "status_undo_toggle" => "Ctrl+U:отмена ",
            "status_quit" => "Ctrl+C:выход",
            _ => "",
        },
        Language::Kazakh => match key {
            "welcome_msg" => {
                "DSX Code — DeepSeek V4 негізіндегі ИИ кодинг агенті.\nТапсырманы енгізіп, Enter басыңыз. Шығу үшін Ctrl+C басыңыз."
            }
            "sidebar_title" => " 📁 ЖҰМЫС АЙМАҒЫ ",
            "reasoning_title" => " 🧠 ОЙЛАУ ПРОЦЕСІ ",
            "chat_title" => " 💻 КОГНИТИВТІК ЯДРО ",
            "input_title_idle" => " ⌨️  ХАБАРЛАМА ЕНГІЗУ ",
            "input_title_running" => " ⏳ КОГНИТИВТІ СТРИМ ЖҰМЫС ІСТЕУДЕ ",
            "input_title_done" => " ⌨️  ХАБАРЛАМА ЕНГІЗУ ",
            "input_title_error" => " ⚠️  БАЙЛАНЫС ҮЗІЛДІ ",
            "input_auth_title" => " 🔒 ҚАУІПСІЗДІК АВТОРИЗАЦИЯ ШЛҮЗІ ",
            "settings_title" => " ⚙️  ЖҮЙЕЛІК БАПТАУ КОРРЕКТОРЫ ",
            "settings_header_banner" => "  DSX CODE ЖҮЙЕ КОРРЕКТОРЫ  ",
            "settings_header_desc" => {
                "   Бағыттау үшін [↑] / [↓] бағыттауыштарын, өзгерту үшін [←] / [→], қайту үшін [Esc] басыңыз."
            }
            "settings_opt_security" => "ҚАУІПСІЗДІК ҚОРҒАУ РЕЖИМІ:   ",
            "settings_opt_model" => "НЕГІЗГІ ИИ МОДЕЛІ:         ",
            "settings_opt_sidebar" => "ФАЙЛДАР БҮЙІРЛІК ПАНЕЛІ:   ",
            "settings_opt_language" => "ЖҮЙЕ ТІЛІ СИПАТТАМАСЫ:     ",
            "settings_opt_clear" => "БЕЛСЕНДІ ЧАТ ТАРИХЫН ЖОЮ:  ",
            "settings_clear_action" => " [ ЧАТ ТАРИХЫН ТАЗАРТУ ҮШІН ENTER БАСЫҢЫЗ ] ",
            "telemetry_title" => "   КИБЕРНЕТИКАЛЫҚ ТЕЛЕМЕТРИЯ СТАТИСТИКАСЫ:",
            "telemetry_db" => "    ▸ SQLite Персистенттілік схемасы: Белсенді (~/.dsx/sessions.db)",
            "telemetry_cost" => "    ▸ Сессияның жинақталған құны: ",
            "telemetry_tokens" => "    ▸ Жалпы сұраныс токендері:     ",
            "diff_title" => " 🔍 БЕЛСЕНДІ ӨЗГЕРІСТЕР (DIFF) ",
            "diff_banner" => "  ӨЗГЕРІСТЕРДІ ИНТЕРФЕЙСТЕ КӨРУ  ",
            "diff_header_desc" => "   Көруді жабу үшін [Ctrl+D] немесе [Esc] басыңыз.",
            "diff_clean" => "   Жұмыс аймағында ешқандай өзгеріс табылған жоқ. (Репозиторий таза)",
            "status_m_toggle" => "m:режим ",
            "status_tree_toggle" => "Ctrl+T:файлдар ",
            "status_settings_toggle" => "Ctrl+S:баптаулар ",
            "status_stop_toggle" => "Ctrl+K:тоқтату ",
            "status_diff_toggle" => "Ctrl+D:дифф ",
            "status_undo_toggle" => "Ctrl+U:қайтару ",
            "status_quit" => "Ctrl+C:шығу",
            _ => "",
        },
        Language::Chinese => match key {
            "welcome_msg" => {
                "DSX Code — 基于 DeepSeek V4 的 AI 编程助手。\n输入任务并按 Enter 回车。按 Ctrl+C 退出。"
            }
            "sidebar_title" => " 📁 工作区 ",
            "reasoning_title" => " 🧠 思维过程 ",
            "chat_title" => " 💻 认知核心 ",
            "input_title_idle" => " ⌨️ 传输输入 ",
            "input_title_running" => " ⏳ 认知流运行中 ",
            "input_title_done" => " ⌨️ 传输输入 ",
            "input_title_error" => " ⚠️ 传输中断 ",
            "input_auth_title" => " 🔒 安全授权网关 ",
            "settings_title" => " ⚙️  系统设置配置器 ",
            "settings_header_banner" => "  DSX CODE 系统设置中心  ",
            "settings_header_desc" => {
                "   使用 [↑] / [↓] 方向键进行导航，[←] / [→] 修改设置参数，按 [Esc] 返回。"
            }
            "settings_opt_security" => "安全防护模式:              ",
            "settings_opt_model" => "核心推理模型:              ",
            "settings_opt_sidebar" => "工作区文件侧边栏:          ",
            "settings_opt_language" => "系统界面显示语言:          ",
            "settings_opt_clear" => "清空当前聊天历史记录:      ",
            "settings_clear_action" => " [ 按 ENTER 回车清空当前聊天 ] ",
            "telemetry_title" => "   系统运行指标监控数据:",
            "telemetry_db" => {
                "    ▸ SQLite 数据库持久化状态: 已激活并成功绑定 (~/.dsx/sessions.db)"
            }
            "telemetry_cost" => "    ▸ 当前会话累计消耗金额:       ",
            "telemetry_tokens" => "    ▸ 已累计统计提示词 Token:     ",
            "diff_title" => " 🔍 活跃工作区差异对比 ",
            "diff_banner" => "  DSX CODE 交互式代码差异分析器  ",
            "diff_header_desc" => "   按 [Ctrl+D] 或 [Esc] 键退出当前差异对比面板。",
            "diff_clean" => "   当前活跃工作区中未检测到任何代码修改。(工作区状态干净)",
            "status_m_toggle" => "m:模式 ",
            "status_tree_toggle" => "Ctrl+T:文件栏 ",
            "status_settings_toggle" => "Ctrl+S:配置面板 ",
            "status_stop_toggle" => "Ctrl+K:停止 ",
            "status_diff_toggle" => "Ctrl+D:差异对比 ",
            "status_undo_toggle" => "Ctrl+U:一键撤销 ",
            "status_quit" => "Ctrl+C:退出",
            _ => "",
        },
    }
}
