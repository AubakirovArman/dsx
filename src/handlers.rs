//! DSX CLI — execution handlers for subcommands.

use std::path::PathBuf;

pub async fn run_plan(
    project_root: PathBuf,
    api_key: String,
    api_base: String,
    task: &str,
    mode: dsx_core::types::PermissionMode,
) -> anyhow::Result<()> {
    let config = dsx_agent::AgentConfig {
        project_root,
        api_key,
        api_base,
        max_iterations: 3,
        mode,
        approval_tx: None,
    };
    println!("Planning agent executing...");
    let outcome = dsx_agent::run(task, &config).await?;
    println!();
    println!("── Plan summary ({iterations} iterations) ──", iterations = outcome.iterations);
    if let Some(ref ans) = outcome.answer {
        println!("{ans}");
    }
    Ok(())
}

pub async fn run_edit(
    project_root: PathBuf,
    api_key: String,
    api_base: String,
    task: &str,
    mode: dsx_core::types::PermissionMode,
) -> anyhow::Result<()> {
    let config = dsx_agent::AgentConfig {
        project_root,
        api_key,
        api_base,
        max_iterations: 15,
        mode,
        approval_tx: None,
    };
    println!("Running agent...");
    let outcome = dsx_agent::run(task, &config).await?;
    println!();
    println!("── Answer ({iterations} iterations) ──", iterations = outcome.iterations);
    if let Some(ref ans) = outcome.answer {
        println!("{ans}");
    }
    Ok(())
}

pub async fn list_sessions(project_root: &PathBuf) {
    let db_path = project_root.join(".dsx").join("sessions.db");
    match dsx_memory::open(&db_path).await {
        Ok(pool) => {
            let sm = dsx_session::SessionManager::new(pool);
            match sm.list(20).await {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        println!("No sessions yet.");
                    } else {
                        println!("Recent sessions:");
                        for s in &sessions {
                            println!(
                                "  {}  {}  {}  {} msgs",
                                &s.id[..8.min(s.id.len())],
                                s.mode,
                                &s.created_at[..19],
                                s.message_count,
                            );
                        }
                    }
                }
                Err(e) => println!("Error: {e}"),
            }
        }
        Err(e) => println!("DB error: {e}"),
    }
}
