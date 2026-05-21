//! DSX CLI — execution handlers for subcommands.

use std::path::{Path, PathBuf};

pub async fn run_plan(
    project_root: PathBuf,
    api_key: String,
    api_base: String,
    task: &str,
    mode: dsx_core::types::PermissionMode,
    allow_wide_scope: bool,
) -> anyhow::Result<()> {
    let scope =
        crate::agent_preflight::prepare_agent_start_scope(&project_root, task, allow_wide_scope)?;
    crate::cli_context_budget::preflight_cli_context_budget(&project_root, task).await?;
    let config = dsx_agent::AgentConfig {
        project_root,
        api_key,
        api_base,
        max_iterations: 3,
        mode,
        approval_tx: None,
    };
    println!("Planning agent executing...");
    print_cli_scope(&scope);
    let outcome = dsx_agent::run(task, &config).await?;
    println!();
    println!(
        "── Plan summary ({iterations} iterations) ──",
        iterations = outcome.iterations
    );
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
    allow_wide_scope: bool,
) -> anyhow::Result<()> {
    let scope =
        crate::agent_preflight::prepare_agent_start_scope(&project_root, task, allow_wide_scope)?;
    crate::cli_context_budget::preflight_cli_context_budget(&project_root, task).await?;
    let config = dsx_agent::AgentConfig {
        project_root,
        api_key,
        api_base,
        max_iterations: 15,
        mode,
        approval_tx: None,
    };
    println!("Running agent...");
    print_cli_scope(&scope);
    let outcome = dsx_agent::run(task, &config).await?;
    println!();
    println!(
        "── Answer ({iterations} iterations) ──",
        iterations = outcome.iterations
    );
    if let Some(ref ans) = outcome.answer {
        println!("{ans}");
    }
    Ok(())
}

pub async fn run_eval(
    project_root: PathBuf,
    api_key: Option<String>,
    api_base: String,
    tasks_file: PathBuf,
    mode: dsx_core::types::PermissionMode,
    no_agent: bool,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&tasks_file)?;
    let tasks = parse_eval_tasks(&content)?;
    let run_agent = !no_agent;
    let key = api_key.unwrap_or_default();
    if run_agent && key.is_empty() {
        anyhow::bail!(
            "Set DEEPSEEK_API_KEY/use --api-key or pass --no-agent for verification-only eval"
        );
    }

    let runner = dsx_eval::EvalRunner::with_config(dsx_eval::EvalConfig {
        project_root,
        api_key: key,
        api_base,
        mode,
        max_iterations: 15,
        run_agent,
        command_timeout_secs: 300,
    });
    let summary = runner.run_suite(&tasks).await?;

    println!(
        "Eval summary: {}/{} passed, {} failed, cost ${:.4}",
        summary.passed, summary.total, summary.failed, summary.total_cost_usd
    );
    for result in &summary.results {
        let status = if result.success { "PASS" } else { "FAIL" };
        println!(
            "  {status} {}  iter={}  tests={:?}  cost=${:.4}",
            result.task_id, result.iterations, result.tests_passed, result.cost_usd
        );
        for missing in &result.missing_expected {
            println!("    - {missing}");
        }
    }

    if summary.failed > 0 {
        anyhow::bail!("eval failed: {} task(s) failed", summary.failed);
    }
    Ok(())
}

pub async fn run_index_build(project_root: &Path) -> anyhow::Result<()> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    let count = dsx_index::build_symbol_index(project_root, &pool).await?;
    println!("Indexed {count} symbols into {}", db_path.display());
    Ok(())
}

pub async fn run_index_search(project_root: &Path, query: &str, limit: u32) -> anyhow::Result<()> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    let symbols = dsx_index::search_symbols(project_root, &pool, query, limit).await?;
    let files = dsx_index::search_files(project_root, query, limit as usize)?;

    println!("Symbols:");
    if symbols.is_empty() {
        println!("  (none)");
    } else {
        for symbol in &symbols {
            println!(
                "  {}:{}  {} {}  {}",
                symbol.path, symbol.start_line, symbol.kind, symbol.name, symbol.signature
            );
        }
    }

    println!("File matches:");
    if files.is_empty() {
        println!("  (none)");
    } else {
        for file_match in &files {
            println!(
                "  {}:{}  {}",
                file_match.path, file_match.line, file_match.text
            );
        }
    }
    Ok(())
}

pub fn run_scope_preview(project_root: &Path, task: &str) {
    let scope = crate::task_scope::resolve_task_scope(project_root, task);
    println!("Task scope preview:");
    println!("  Task: {}", task_preview(task));
    println!("  Launch: {}", scope.launch_label);
    println!("  Active: {}", scope.active_label);
    println!(
        "  Status: {}",
        if scope.narrowed { "NARROWED" } else { "WIDE" }
    );
    println!(
        "  Reason: {}",
        if scope.narrowed {
            "Task selected a subfolder; tools and indexing will be locked there."
        } else {
            "No explicit subfolder was selected; launch workspace remains active."
        }
    );
    if !scope.narrowed {
        println!("  Warning: add an explicit folder like ./1234 to narrow scope.");
    }
    println!(
        "  Active exists: {}",
        if scope.active_root.exists() {
            "yes"
        } else {
            "no"
        }
    );
}

fn print_cli_scope(scope: &crate::task_scope::ResolvedTaskScope) {
    if scope.narrowed {
        println!("Scope: {}", scope.active_label);
    }
}

pub async fn run_mcp_list(command: &str, args: &[String]) -> anyhow::Result<()> {
    let mut client = dsx_mcp::McpClient::connect_stdio(command, args).await?;
    let tools = client.list_tools().await?;
    if tools.is_empty() {
        println!("No MCP tools exposed.");
    } else {
        println!("MCP tools:");
        for tool in &tools {
            let description = tool.description.as_deref().unwrap_or("");
            println!("  {}  {}", tool.name, description);
        }
    }
    client.shutdown().await?;
    Ok(())
}

pub async fn run_mcp_call(
    command: &str,
    args: &[String],
    tool: &str,
    arguments_json: &str,
) -> anyhow::Result<()> {
    let arguments = serde_json::from_str::<serde_json::Value>(arguments_json)?;
    let mut client = dsx_mcp::McpClient::connect_stdio(command, args).await?;
    let result = client.call_tool(tool, arguments).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    client.shutdown().await?;
    Ok(())
}

fn parse_eval_tasks(content: &str) -> anyhow::Result<Vec<dsx_eval::EvalTask>> {
    match serde_json::from_str::<Vec<dsx_eval::EvalTask>>(content) {
        Ok(tasks) => Ok(tasks),
        Err(vec_err) => match serde_json::from_str::<dsx_eval::EvalTask>(content) {
            Ok(task) => Ok(vec![task]),
            Err(task_err) => Err(anyhow::anyhow!(
                "failed to parse eval tasks as array ({vec_err}) or object ({task_err})"
            )),
        },
    }
}

pub fn task_preview(task: &str) -> String {
    const MAX_CHARS: usize = 240;
    let cleaned = dsx_agent::brief::clean_task_input(task);
    let mut preview: String = cleaned.chars().take(MAX_CHARS).collect();
    if cleaned.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }
    preview
}

pub async fn list_sessions(project_root: &Path) {
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
