//! DSX Eval — local benchmark runner for coding-agent regression tests.
//!
//! The runner can execute a task through `dsx-agent`, then verify expected files,
//! content snippets, and an optional test command inside the workspace.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub expected_changes: Vec<String>,
    #[serde(default)]
    pub expected_files: Vec<String>,
    #[serde(default)]
    pub expected_contains: Vec<FileExpectation>,
    #[serde(default)]
    pub test_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileExpectation {
    pub path: String,
    pub contains: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EvalResult {
    pub task_id: String,
    pub success: bool,
    pub patch_applied: bool,
    pub expected_passed: bool,
    pub missing_expected: Vec<String>,
    pub tests_passed: Option<bool>,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub answer: Option<String>,
    pub iterations: usize,
    pub token_usage: TokenUsage,
    pub cost_usd: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EvalSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_cost_usd: f64,
    pub results: Vec<EvalResult>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_hit_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct EvalConfig {
    pub project_root: PathBuf,
    pub api_key: String,
    pub api_base: String,
    pub mode: dsx_core::types::PermissionMode,
    pub max_iterations: usize,
    pub run_agent: bool,
    pub command_timeout_secs: u64,
}

pub struct EvalRunner {
    config: Option<EvalConfig>,
}

impl EvalRunner {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn with_config(config: EvalConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub async fn run(&self, task: &EvalTask) -> anyhow::Result<EvalResult> {
        let start = std::time::Instant::now();
        let mut result = EvalResult {
            task_id: task.id.clone(),
            ..Default::default()
        };

        if let Some(config) = &self.config
            && config.run_agent
        {
            if config.api_key.is_empty() {
                anyhow::bail!(
                    "eval task '{}' needs an API key when run_agent=true",
                    task.id
                );
            }
            let agent_config = dsx_agent::AgentConfig {
                project_root: config.project_root.clone(),
                api_key: config.api_key.clone(),
                api_base: config.api_base.clone(),
                max_iterations: config.max_iterations,
                mode: config.mode,
                approval_tx: None,
            };
            let outcome = dsx_agent::run(&task.description, &agent_config).await?;
            result.answer = outcome.answer;
            result.iterations = outcome.iterations;
            result.token_usage = TokenUsage {
                prompt_tokens: outcome.total_prompt_tokens,
                completion_tokens: outcome.total_completion_tokens,
                reasoning_tokens: outcome.total_reasoning_tokens,
                cache_hit_tokens: 0,
            };
            result.cost_usd = outcome.estimated_cost_usd;
            result.patch_applied = outcome.tool_results.iter().any(|tool| tool.success);
        }

        let root = self
            .config
            .as_ref()
            .map(|config| config.project_root.as_path())
            .unwrap_or_else(|| Path::new("."));
        let verification = verify_expectations(root, task);
        result.expected_passed = verification.is_empty();
        result.missing_expected = verification;

        if let Some(command) = task.test_command.as_deref() {
            let timeout_secs = self
                .config
                .as_ref()
                .map(|config| config.command_timeout_secs)
                .unwrap_or(300);
            let run = dsx_sandbox::run("sh", &["-lc", command], root, timeout_secs).await?;
            result.exit_code = run.exit_code;
            result.tests_passed = Some(run.exit_code == Some(0));
        }

        let tests_ok = result.tests_passed.unwrap_or(true);
        result.success = result.expected_passed && tests_ok;
        result.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    pub async fn run_suite(&self, tasks: &[EvalTask]) -> anyhow::Result<EvalSummary> {
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            results.push(self.run(task).await?);
        }
        let passed = results.iter().filter(|result| result.success).count();
        let total_cost_usd = results.iter().map(|result| result.cost_usd).sum();
        Ok(EvalSummary {
            total: results.len(),
            passed,
            failed: results.len().saturating_sub(passed),
            total_cost_usd,
            results,
        })
    }
}

impl Default for EvalRunner {
    fn default() -> Self {
        Self::new()
    }
}

fn verify_expectations(root: &Path, task: &EvalTask) -> Vec<String> {
    let mut missing = Vec::new();

    for path in task
        .expected_files
        .iter()
        .chain(task.expected_changes.iter())
    {
        if !root.join(path).exists() {
            missing.push(format!("missing file: {path}"));
        }
    }

    for expected in &task.expected_contains {
        let full_path = root.join(&expected.path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) if content.contains(&expected.contains) => {}
            Ok(_) => missing.push(format!("missing content in {}", expected.path)),
            Err(e) => missing.push(format!("cannot read {}: {e}", expected.path)),
        }
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn eval_runner_verifies_files_and_command() {
        let tmp = std::env::temp_dir().join("dsx_eval_test_ok");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("src/main.rs"), "fn main() {}\n").unwrap();

        let runner = EvalRunner::with_config(EvalConfig {
            project_root: tmp.clone(),
            api_key: String::new(),
            api_base: "http://localhost".into(),
            mode: dsx_core::types::PermissionMode::Yolo,
            max_iterations: 1,
            run_agent: false,
            command_timeout_secs: 10,
        });
        let task = EvalTask {
            id: "local".into(),
            description: "verify".into(),
            expected_changes: Vec::new(),
            expected_files: vec!["src/main.rs".into()],
            expected_contains: vec![FileExpectation {
                path: "src/main.rs".into(),
                contains: "fn main".into(),
            }],
            test_command: Some("test -f src/main.rs".into()),
        };

        let result = runner.run(&task).await.unwrap();
        assert!(result.success);
        assert_eq!(result.tests_passed, Some(true));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn eval_runner_reports_missing_expectations() {
        let tmp = std::env::temp_dir().join("dsx_eval_test_missing");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let runner = EvalRunner::with_config(EvalConfig {
            project_root: tmp.clone(),
            api_key: String::new(),
            api_base: "http://localhost".into(),
            mode: dsx_core::types::PermissionMode::Yolo,
            max_iterations: 1,
            run_agent: false,
            command_timeout_secs: 10,
        });
        let task = EvalTask {
            id: "missing".into(),
            description: "verify".into(),
            expected_changes: vec!["missing.txt".into()],
            expected_files: Vec::new(),
            expected_contains: Vec::new(),
            test_command: None,
        };

        let result = runner.run(&task).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.missing_expected, vec!["missing file: missing.txt"]);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
