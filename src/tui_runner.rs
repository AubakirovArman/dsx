//! Ratatui runtime loop.

use crate::tui_keys::KeyOutcome;
use crate::tui_state::{SharedApp, configure_initial_app, load_recent_history, start_indexing};
use crossterm::event::{self, Event, KeyEventKind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub async fn run_tui(
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
    let app: SharedApp = Arc::new(Mutex::new(dsx_tui::App::new()));
    let history = load_recent_history(session_id.clone(), pool.clone()).await;
    configure_initial_app(
        &app,
        &project_root,
        initial_mode,
        api_base,
        api_key.clone(),
        history,
    );
    start_indexing(app.clone(), project_root.clone(), pool.clone(), &rt);

    let result = event_loop(
        &mut terminal,
        app,
        project_root,
        api_key,
        session_id,
        pool,
        &rt,
    )
    .await;

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    result
}

async fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: SharedApp,
    project_root: PathBuf,
    api_key: String,
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
    rt: &tokio::runtime::Handle,
) -> anyhow::Result<()>
where
    <B as ratatui::backend::Backend>::Error: Send + Sync + 'static,
{
    loop {
        {
            let app = app.lock().unwrap();
            terminal.draw(|frame| app.draw(frame))?;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        let outcome =
            crate::tui_keys::handle_key(key, &app, &project_root, &api_key, &session_id, &pool, rt)
                .await?;
        if outcome == KeyOutcome::Quit {
            break;
        }
    }
    Ok(())
}
