//! DSX Code — terminal coding agent entrypoint.

pub mod cli;
pub mod handlers;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

use cli::{CliArgs, Command, IndexAction, McpAction, WorkspaceAction};
use handlers::{
    list_sessions, run_edit, run_eval, run_index_build, run_index_search, run_mcp_call,
    run_mcp_list, run_plan, task_preview,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CliArgs::parse();

    let project_root = std::fs::canonicalize(&cli.workspace).unwrap_or(cli.workspace);
    let app_config = match dsx_config::load_for_project(&project_root) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: failed to load config: {e}");
            dsx_config::AppConfig::default()
        }
    };

    let mode_name = cli
        .mode
        .as_deref()
        .unwrap_or(app_config.app.default_mode.as_str());
    let mode = dsx_core::types::PermissionMode::parse(mode_name)
        .unwrap_or(dsx_core::types::PermissionMode::Ask);

    let api_key = cli
        .api_key
        .clone()
        .or_else(|| std::env::var(&app_config.provider.api_key_env).ok())
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok());
    let api_base = cli
        .api_base
        .clone()
        .unwrap_or_else(|| app_config.provider.openai_base_url.clone());

    match cli.command {
        None | Some(Command::Interactive) => {
            let key = api_key.unwrap_or_default();
            // Init SQLite session
            let db_path = project_root.join(".dsx").join("sessions.db");
            let (pool, sid) = match dsx_memory::open(&db_path).await {
                Ok(pool) => {
                    let sm = dsx_session::SessionManager::new(pool.clone());
                    match sm
                        .create(&project_root.display().to_string(), mode.as_str())
                        .await
                    {
                        Ok(s) => (Some(pool), Some(s.id)),
                        Err(_) => (Some(pool), None),
                    }
                }
                Err(_) => (None, None),
            };
            run_tui(project_root, key, api_base, mode, sid, pool).await?;
        }

        Some(Command::Plan { task }) => {
            let desc = task.join(" ");
            let key = match api_key {
                Some(k) => k,
                None => {
                    println!("(Set DEEPSEEK_API_KEY or use --api-key)");
                    return Ok(());
                }
            };
            println!("Planning: {}", task_preview(&desc));
            run_plan(project_root, key, api_base, &desc, mode).await?;
        }

        Some(Command::Edit { task }) => {
            let desc = task.join(" ");
            let key = match api_key {
                Some(k) => k,
                None => {
                    println!("(Set DEEPSEEK_API_KEY or use --api-key)");
                    return Ok(());
                }
            };
            println!("Editing: {}", task_preview(&desc));
            run_edit(project_root, key, api_base, &desc, mode).await?;
        }

        Some(Command::Eval {
            tasks_file,
            no_agent,
        }) => {
            run_eval(project_root, api_key, api_base, tasks_file, mode, no_agent).await?;
        }

        Some(Command::Index { action }) => match action {
            IndexAction::Build => {
                run_index_build(&project_root).await?;
            }
            IndexAction::Search { query, limit } => {
                run_index_search(&project_root, &query, limit).await?;
            }
        },

        Some(Command::Mcp { action }) => match action {
            McpAction::List { command, args } => {
                run_mcp_list(&command, &args).await?;
            }
            McpAction::Call {
                tool,
                arguments_json,
                command,
                args,
            } => {
                run_mcp_call(&command, &args, &tool, &arguments_json).await?;
            }
        },

        Some(Command::Workspace { action }) => match action {
            None | Some(WorkspaceAction::List) => {
                list_sessions(&project_root).await;
            }
            Some(WorkspaceAction::Resume { id }) => {
                let key = match api_key {
                    Some(k) => k,
                    None => {
                        println!("(Set DEEPSEEK_API_KEY or use --api-key)");
                        return Ok(());
                    }
                };
                let db_path = project_root.join(".dsx").join("sessions.db");
                match dsx_memory::open(&db_path).await {
                    Ok(pool) => {
                        let sm = dsx_session::SessionManager::new(pool.clone());
                        match sm.get(&id).await {
                            Ok(Some(s)) => {
                                let session_mode =
                                    dsx_core::types::PermissionMode::parse(&s.mode).unwrap_or(mode);
                                println!("Resuming session {}...", s.id);
                                run_tui(
                                    project_root,
                                    key,
                                    api_base,
                                    session_mode,
                                    Some(s.id),
                                    Some(pool),
                                )
                                .await?;
                            }
                            _ => {
                                println!("Error: Session with ID '{}' not found.", id);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error: Failed to open sessions database: {e}");
                    }
                }
            }
        },
    }

    Ok(())
}

// ── TUI mode ────────────────────────────────────────────────────────

async fn run_tui(
    project_root: PathBuf,
    api_key: String,
    api_base: String,
    initial_mode: dsx_core::types::PermissionMode,
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
) -> anyhow::Result<()> {
    use ratatui::{Terminal, backend::CrosstermBackend};

    let rt = tokio::runtime::Handle::current();

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(std::io::stderr());
    let mut terminal = Terminal::new(backend)?;

    let app = Arc::new(Mutex::new(dsx_tui::App::new()));
    let history_events = if let (Some(sid), Some(p)) = (session_id.clone(), pool.clone()) {
        let sm = dsx_session::SessionManager::new(p);
        sm.get_events(&sid).await.ok().map(|events| (sid, events))
    } else {
        None
    };

    // Set custom API base URL dynamically
    {
        let mut a = app.lock().unwrap();
        a.api_base = api_base;
    }

    // Set initial project info, file tree, and mode
    {
        let mut a = app.lock().unwrap();
        a.mode = initial_mode.as_str().to_string();
        a.add_message("system", &format!("Project: {}", project_root.display()));
        a.add_message(
            "system",
            &format!(
                "Mode: {} — {}",
                initial_mode.as_str(),
                initial_mode.description()
            ),
        );

        // Load history if pool and session_id are provided
        if let Some((sid, events)) = history_events {
            a.add_message("system", &format!("Session ID: {}", sid));
            if !events.is_empty() {
                a.add_message(
                    "system",
                    &format!(
                        "✓ Loaded {} historical message events from SQLite.",
                        events.len()
                    ),
                );
                for e in events {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&e.data_json) {
                        if e.type_ == "user_msg" {
                            if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                                a.add_message("user", content);
                            }
                        } else if e.type_ == "assistant_msg" {
                            if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                                a.add_message("assistant", content);
                            }
                            if let Some(cost) = data.get("cost").and_then(|v| v.as_f64()) {
                                a.cost = cost;
                            }
                            if let Some(tokens) = data.get("tokens").and_then(|v| v.as_u64()) {
                                a.tokens = tokens;
                            }
                        }
                    }
                }
            }
        }

        // Auto-init git if needed
        if !project_root.join(".git").exists() {
            let result = std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(&project_root)
                .output();
            match result {
                Ok(o) if o.status.success() => {
                    a.add_message("system", "✓ git init (for checkpoints)");
                }
                _ => {
                    a.add_message("system", "⚠ no git repo — checkpoints disabled");
                }
            }
        }
        // Populate file tree
        if let Ok(files) = dsx_index::scan_project(&project_root) {
            a.file_tree = files.into_iter().take(50).collect();
        }

        // Trigger semantic indexing in background
        if let Some(ref p) = pool {
            let p_copy = p.clone();
            let root_copy = project_root.clone();
            let app_copy = app.clone();
            rt.spawn(async move {
                if let Ok(count) = dsx_index::build_symbol_index(&root_copy, &p_copy).await {
                    let mut a = app_copy.lock().unwrap();
                    a.add_message("system", &format!("✓ Semantic Indexing complete: {count} structural symbols indexed in SQLite."));
                }
            });
        }
    }

    loop {
        // Draw
        {
            let a = app.lock().unwrap();
            terminal.draw(|f| a.draw(f))?;
        }

        // Poll for input with a small timeout
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let ev = event::read()?;
        match ev {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                // Intercept keys if there is a pending tool authorization request
                let has_pending_approval = {
                    let a = app.lock().unwrap();
                    a.pending_approval.is_some()
                };

                if has_pending_approval {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            let mut a = app.lock().unwrap();
                            if let Some(approval) = a.pending_approval.take() {
                                let _ = approval.tx.send(true);
                                a.add_message(
                                    "system",
                                    "🔐 Authorization: APPROVED (tool executing...)",
                                );
                            }
                            continue;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            let mut a = app.lock().unwrap();
                            if let Some(approval) = a.pending_approval.take() {
                                let _ = approval.tx.send(false);
                                a.add_message("system", "🔒 Authorization: DENIED (tool blocked)");
                            }
                            continue;
                        }
                        KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            break;
                        }
                        _ => {
                            // Suppress other keys while authorization is pending
                            continue;
                        }
                    }
                }

                let is_diff_active = {
                    let a = app.lock().unwrap();
                    a.show_diff
                };

                if is_diff_active {
                    match key.code {
                        KeyCode::Esc => {
                            let mut a = app.lock().unwrap();
                            a.show_diff = false;
                            continue;
                        }
                        KeyCode::Char('d')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            let mut a = app.lock().unwrap();
                            a.show_diff = false;
                            continue;
                        }
                        KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            break;
                        }
                        _ => {
                            // Suppress other inputs while diff is active
                            continue;
                        }
                    }
                }

                let is_settings_active = {
                    let a = app.lock().unwrap();
                    a.show_settings
                };

                if is_settings_active {
                    match key.code {
                        KeyCode::Esc => {
                            let mut a = app.lock().unwrap();
                            a.show_settings = false;
                            continue;
                        }
                        KeyCode::Char('s')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            let mut a = app.lock().unwrap();
                            a.show_settings = false;
                            continue;
                        }
                        KeyCode::Up => {
                            let mut a = app.lock().unwrap();
                            a.settings_cursor = a.settings_cursor.saturating_sub(1);
                            continue;
                        }
                        KeyCode::Down => {
                            let mut a = app.lock().unwrap();
                            a.settings_cursor = (a.settings_cursor + 1).min(6); // 7 options total (0..=6)
                            continue;
                        }
                        KeyCode::Left | KeyCode::Right => {
                            let mut a = app.lock().unwrap();
                            match a.settings_cursor {
                                0 => {
                                    // Toggle Protection Mode
                                    let current = dsx_core::types::PermissionMode::parse(&a.mode)
                                        .unwrap_or(dsx_core::types::PermissionMode::Ask);
                                    let all = dsx_core::types::PermissionMode::all();
                                    let idx = all.iter().position(|x| *x == current).unwrap_or(2);
                                    let offset = if matches!(key.code, KeyCode::Left) {
                                        all.len() - 1
                                    } else {
                                        1
                                    };
                                    let next = all[(idx + offset) % all.len()];
                                    a.mode = next.as_str().to_string();
                                }
                                1 => {
                                    // Toggle active model
                                    if a.model == "v4-pro" {
                                        a.model = "v4-flash".to_string();
                                    } else {
                                        a.model = "v4-pro".to_string();
                                    }
                                }
                                2 => {
                                    // Toggle file tree sidebar
                                    a.show_file_tree = !a.show_file_tree;
                                }
                                3 => {
                                    // Toggle interface language (i18n)
                                    let all = dsx_tui::Language::all();
                                    let idx = all.iter().position(|x| *x == a.lang).unwrap_or(0);
                                    let offset = if matches!(key.code, KeyCode::Left) {
                                        all.len() - 1
                                    } else {
                                        1
                                    };
                                    let next = all[(idx + offset) % all.len()];
                                    a.lang = next;

                                    // Localize the change log message
                                    let change_log = match next {
                                        dsx_tui::Language::Russian => {
                                            "Язык интерфейса изменен на Русский."
                                        }
                                        dsx_tui::Language::Kazakh => {
                                            "Интерфейс тілі Қазақша болып өзгертілді."
                                        }
                                        dsx_tui::Language::Chinese => {
                                            "界面显示语言已成功切换为 中文。"
                                        }
                                        dsx_tui::Language::English => {
                                            "Interface language successfully changed to English."
                                        }
                                    };
                                    a.add_message("system", change_log);
                                }
                                4 => {
                                    // Toggle API Base URL
                                    let presets = [
                                        "https://api.deepseek.com",
                                        "http://localhost:11434/v1",
                                        "http://localhost:8000/v1",
                                        "https://api.openai.com/v1",
                                    ];
                                    let idx =
                                        presets.iter().position(|x| *x == a.api_base).unwrap_or(0);
                                    let offset = if matches!(key.code, KeyCode::Left) {
                                        presets.len() - 1
                                    } else {
                                        1
                                    };
                                    let next = presets[(idx + offset) % presets.len()];
                                    a.api_base = next.to_string();

                                    let change_log = match a.lang {
                                        dsx_tui::Language::Russian => {
                                            format!("Адрес API Endpoint изменен на: {}", next)
                                        }
                                        dsx_tui::Language::Kazakh => {
                                            format!("API Endpoint мекені ауыстырылды: {}", next)
                                        }
                                        dsx_tui::Language::Chinese => {
                                            format!("API 服务基准地址已切换为: {}", next)
                                        }
                                        dsx_tui::Language::English => {
                                            format!("API Endpoint base changed to: {}", next)
                                        }
                                    };
                                    a.add_message("system", &change_log);
                                }
                                5 => {
                                    // API Key informative notice
                                    let msg = match a.lang {
                                        dsx_tui::Language::Russian => {
                                            "🔑 Системный лог: Ключ авторизации API Key надежно загружен из системного окружения."
                                        }
                                        dsx_tui::Language::Kazakh => {
                                            "🔑 Жүйелік журнал: API авторизация кілті жүйелік ортадан қауіпсіз түрде жүктелді."
                                        }
                                        dsx_tui::Language::Chinese => {
                                            "🔑 系统日志: API 授权密钥已从系统环境安全变量中加载完毕。"
                                        }
                                        dsx_tui::Language::English => {
                                            "🔑 System Log: API Key is securely loaded from system environmental variables."
                                        }
                                    };
                                    a.add_message("system", msg);
                                }
                                _ => {}
                            }
                            continue;
                        }
                        KeyCode::Enter => {
                            let mut a = app.lock().unwrap();
                            if a.settings_cursor == 6 {
                                a.messages.clear();
                                let clear_msg = match a.lang {
                                    dsx_tui::Language::Russian => {
                                        "🧹 Системный лог: Когнитивная история чата очищена."
                                    }
                                    dsx_tui::Language::Kazakh => {
                                        "🧹 Жүйелік журнал: Чат тарихы толығымен тазартылды."
                                    }
                                    dsx_tui::Language::Chinese => {
                                        "🧹 系统日志: 当前会话聊天历史记录已成功清除。"
                                    }
                                    dsx_tui::Language::English => {
                                        "🧹 System Log: Conversational core history wiped."
                                    }
                                };
                                a.add_message("system", clear_msg);
                            }
                            continue;
                        }
                        KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            break;
                        }
                        _ => {
                            continue;
                        }
                    }
                }

                match key.code {
                    KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break;
                    }
                    KeyCode::Enter => {
                        let task: String;
                        let api_key_copy: String;
                        let project_root_copy: PathBuf;
                        let current_mode: dsx_core::types::PermissionMode;
                        {
                            let mut a = app.lock().unwrap();
                            task = a.input.clone();
                            a.input.clear();
                            a.scroll_offset = 0;
                            if task.trim().is_empty() {
                                continue;
                            }
                            current_mode = dsx_core::types::PermissionMode::parse(&a.mode)
                                .unwrap_or(dsx_core::types::PermissionMode::Ask);
                            a.add_message("user", &task);
                            a.agent_task = dsx_tui::AgentTask::Running(task.clone());
                            api_key_copy = api_key.clone();
                            project_root_copy = project_root.clone();
                        }

                        // Persist user message to SQLite in background
                        if let (Some(ref sid), Some(ref p)) = (session_id.clone(), pool.clone()) {
                            let sm = dsx_session::SessionManager::new(p.clone());
                            let sid_copy = sid.clone();
                            let task_copy = task.clone();
                            rt.spawn(async move {
                                let _ = sm
                                    .record_event(
                                        &sid_copy,
                                        "user_msg",
                                        &serde_json::json!({ "content": task_copy }),
                                    )
                                    .await;
                            });
                        }

                        let api_base_copy = {
                            let a = app.lock().unwrap();
                            a.api_base.clone()
                        };

                        // Spawn agent with streaming and approvals
                        let (tx, mut rx) = mpsc::unbounded_channel();
                        let (approval_tx, mut approval_rx) = mpsc::unbounded_channel();

                        let approval_tx_opt = Some(approval_tx);
                        rt.spawn(async move {
                            let config = dsx_agent::AgentConfig {
                                project_root: project_root_copy,
                                api_key: api_key_copy,
                                api_base: api_base_copy,
                                max_iterations: 15,
                                mode: current_mode,
                                approval_tx: approval_tx_opt,
                            };
                            let _ = dsx_agent::run_streaming(&task, &config, tx).await;
                        });

                        // Monitor approval requests and update App state in real-time
                        let app_loop = app.clone();
                        rt.spawn(async move {
                            while let Some(req) = approval_rx.recv().await {
                                let mut a = app_loop.lock().unwrap();
                                a.pending_approval = Some(dsx_tui::PendingApproval {
                                    tool_name: req.tool_name,
                                    arguments: req.arguments,
                                    tx: req.tx,
                                });
                            }
                        });

                        // Read streaming events and update UI until channel closes
                        let app_loop2 = app.clone();
                        let session_id_opt = session_id.clone();
                        let pool_opt = pool.clone();
                        let rt_copy = rt.clone();
                        rt.spawn(async move {
                            while let Some(event) = rx.recv().await {
                                let tui_event = convert_event(&event);
                                let mut a = app_loop2.lock().unwrap();
                                a.handle_stream_event(&tui_event);
                            }
                            // Agent finished
                            let mut a = app_loop2.lock().unwrap();
                            a.agent_task = dsx_tui::AgentTask::Done("ready".into());

                            // Persist assistant message to SQLite
                            if let Some(last_msg) = a.messages.last() {
                                if last_msg.role == "assistant" {
                                    if let (Some(sid), Some(p)) =
                                        (session_id_opt.clone(), pool_opt.clone())
                                    {
                                        let sm = dsx_session::SessionManager::new(p);
                                        let content = last_msg.content.clone();
                                        let cost = a.cost;
                                        let tokens = a.tokens;
                                        rt_copy.spawn(async move {
                                            let _ = sm
                                                .record_event(
                                                    &sid,
                                                    "assistant_msg",
                                                    &serde_json::json!({
                                                        "content": content,
                                                        "cost": cost,
                                                        "tokens": tokens,
                                                    }),
                                                )
                                                .await;
                                        });
                                    }
                                }
                            }
                        });
                    }
                    KeyCode::Char('t')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        let mut a = app.lock().unwrap();
                        a.show_file_tree = !a.show_file_tree;
                    }
                    KeyCode::Char('s')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        let mut a = app.lock().unwrap();
                        a.show_settings = !a.show_settings;
                    }
                    KeyCode::Char('d')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        let mut a = app.lock().unwrap();
                        if !a.show_diff {
                            if let Ok(d) = dsx_git::diff(&project_root) {
                                a.current_diff = d;
                            } else {
                                a.current_diff = "Error: Failed to fetch git diff.".into();
                            }
                        }
                        a.show_diff = !a.show_diff;
                    }
                    KeyCode::Char('u')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        let mut a = app.lock().unwrap();
                        match dsx_git::rollback(&project_root) {
                            Ok(msg) => {
                                a.add_message("system", &format!("⏪ Workspace Reverted: {}", msg));
                                // Reload file tree
                                if let Ok(files) = dsx_index::scan_project(&project_root) {
                                    a.file_tree = files.into_iter().take(50).collect();
                                }
                            }
                            Err(e) => {
                                a.add_message("error", &format!("🔒 Undo Failed: {e}"));
                            }
                        }
                    }
                    KeyCode::Up => {
                        let mut a = app.lock().unwrap();
                        if a.input.is_empty() {
                            a.scroll_offset = a.scroll_offset.saturating_add(1); // going up increments the scroll offset backwards!
                        }
                    }
                    KeyCode::Down => {
                        let mut a = app.lock().unwrap();
                        if a.input.is_empty() {
                            a.scroll_offset = a.scroll_offset.saturating_sub(1);
                        }
                    }
                    KeyCode::PageUp => {
                        let mut a = app.lock().unwrap();
                        a.scroll_offset = a.scroll_offset.saturating_add(10);
                    }
                    KeyCode::PageDown => {
                        let mut a = app.lock().unwrap();
                        a.scroll_offset = a.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::Char(ch) => {
                        let mut a = app.lock().unwrap();
                        a.input.push(ch);
                        a.scroll_offset = 0;
                    }
                    KeyCode::Backspace => {
                        let mut a = app.lock().unwrap();
                        a.input.pop();
                        a.scroll_offset = 0;
                    }
                    KeyCode::Esc => {
                        let mut a = app.lock().unwrap();
                        a.input.clear();
                        a.scroll_offset = 0;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

// ── Event conversion ────────────────────────────────────────────────

fn convert_event(ev: &dsx_provider::streaming::StreamEvent) -> dsx_tui::AgentStreamEvent {
    match ev {
        dsx_provider::streaming::StreamEvent::Reasoning(r) => {
            dsx_tui::AgentStreamEvent::Reasoning(r.clone())
        }
        dsx_provider::streaming::StreamEvent::Content(c) => {
            dsx_tui::AgentStreamEvent::ContentToken(c.clone())
        }
        dsx_provider::streaming::StreamEvent::ToolCall(tc) => {
            dsx_tui::AgentStreamEvent::ToolResult {
                name: tc.name.clone(),
                success: true,
                summary: format!("requested {}", tc.name),
            }
        }
        dsx_provider::streaming::StreamEvent::ToolResult {
            name,
            success,
            summary,
        } => dsx_tui::AgentStreamEvent::ToolResult {
            name: name.clone(),
            success: *success,
            summary: summary.clone(),
        },
        dsx_provider::streaming::StreamEvent::Finish { .. } => {
            // Finish is handled by Done event separately
            dsx_tui::AgentStreamEvent::Reasoning(String::new())
        }
        dsx_provider::streaming::StreamEvent::Done {
            answer,
            iterations,
            tokens,
            cost,
        } => dsx_tui::AgentStreamEvent::Done {
            answer: answer.clone(),
            iterations: *iterations,
            tokens: *tokens,
            cost: *cost,
        },
        dsx_provider::streaming::StreamEvent::Error(err) => {
            dsx_tui::AgentStreamEvent::Error(err.clone())
        }
    }
}
