//! DSX Sandbox — classified command execution with timeout and output truncation.

use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct RunResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration_ms: u64,
}

const MAX_OUTPUT_BYTES: usize = 131_072; // 128 KiB

/// Run a command in the given working directory with a timeout.
pub async fn run(
    cmd: &str,
    args: &[&str],
    cwd: &std::path::Path,
    timeout_secs: u64,
) -> anyhow::Result<RunResult> {
    let start = std::time::Instant::now();
    let child = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()?;

    let dur = Duration::from_secs(timeout_secs);
    let output = timeout(dur, child.wait_with_output()).await??;

    let duration_ms = start.elapsed().as_millis() as u64;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let truncated = stdout.len() > MAX_OUTPUT_BYTES || stderr.len() > MAX_OUTPUT_BYTES;
    let stdout = truncate(stdout);
    let stderr = truncate(stderr);

    Ok(RunResult {
        exit_code: output.status.code(),
        stdout,
        stderr,
        truncated,
        duration_ms,
    })
}

fn truncate(s: String) -> String {
    if s.len() > MAX_OUTPUT_BYTES {
        let mut t = s[..MAX_OUTPUT_BYTES].to_string();
        t.push_str("\n... [output truncated]");
        t
    } else {
        s
    }
}
