//! CLI handlers that run the agent or eval suite.

use std::path::PathBuf;

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
    let config = agent_config(project_root, api_key, api_base, 3, mode);
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
    let config = agent_config(project_root, api_key, api_base, 15, mode);
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

fn agent_config(
    project_root: PathBuf,
    api_key: String,
    api_base: String,
    max_iterations: usize,
    mode: dsx_core::types::PermissionMode,
) -> dsx_agent::AgentConfig {
    dsx_agent::AgentConfig {
        project_root,
        api_key,
        api_base,
        max_iterations,
        mode,
        approval_tx: None,
    }
}

fn print_cli_scope(scope: &crate::task_scope::ResolvedTaskScope) {
    if scope.narrowed {
        println!("Scope: {}", scope.active_label);
    }
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
