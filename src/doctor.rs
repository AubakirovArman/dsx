//! Workspace readiness diagnostics.

use std::path::{Path, PathBuf};

const MAX_RS_LINES: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLineCount {
    pub path: PathBuf,
    pub lines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

struct Check {
    status: CheckStatus,
    name: &'static str,
    detail: String,
}

pub async fn run_doctor(
    project_root: &Path,
    api_base: &str,
    api_key: Option<&str>,
) -> anyhow::Result<()> {
    let checks = collect_checks(project_root, api_base, api_key).await;
    println!("DSX doctor: {}", project_root.display());
    for check in &checks {
        println!("{} {} - {}", marker(check.status), check.name, check.detail);
    }

    if checks.iter().any(|check| check.status == CheckStatus::Fail) {
        anyhow::bail!("doctor found failing checks");
    }
    Ok(())
}

async fn collect_checks(project_root: &Path, api_base: &str, api_key: Option<&str>) -> Vec<Check> {
    let mut checks = Vec::new();
    checks.push(workspace_check(project_root));
    checks.push(api_check(api_base, api_key));
    checks.push(budget_check());
    checks.push(git_check(project_root));
    checks.push(memory_check(project_root).await);
    checks.push(line_limit_check(project_root));
    checks
}

fn workspace_check(project_root: &Path) -> Check {
    if project_root.is_dir() {
        ok("workspace", "workspace directory exists")
    } else {
        fail(
            "workspace",
            "workspace directory is missing or not a directory",
        )
    }
}

fn api_check(api_base: &str, api_key: Option<&str>) -> Check {
    if api_key.is_some_and(|key| !key.trim().is_empty()) {
        return ok("api", format!("key detected; base={api_base}"));
    }
    if api_base.contains("localhost") || api_base.contains("127.0.0.1") {
        return warn("api", format!("no key detected; local base={api_base}"));
    }
    warn("api", format!("no key detected for base={api_base}"))
}

fn budget_check() -> Check {
    let limits = dsx_agent::budget::current_limits();
    ok(
        "budget",
        format!(
            "token/cost fuse: {}",
            dsx_agent::budget::format_limits(limits)
        ),
    )
}

fn git_check(project_root: &Path) -> Check {
    if project_root.join(".git").is_dir() {
        ok("git", "git repository present")
    } else {
        warn("git", "no .git directory; TUI will attempt git init")
    }
}

async fn memory_check(project_root: &Path) -> Check {
    let db_path = project_root.join(".dsx").join("sessions.db");
    match dsx_memory::open(&db_path).await {
        Ok(pool) => {
            pool.close().await;
            ok("memory", format!("SQLite ready at {}", db_path.display()))
        }
        Err(e) => fail("memory", format!("failed to open SQLite: {e}")),
    }
}

fn line_limit_check(project_root: &Path) -> Check {
    match rust_line_violations(project_root, MAX_RS_LINES) {
        Ok(violations) if violations.is_empty() => ok(
            "line-limit",
            format!("all Rust files <= {MAX_RS_LINES} lines"),
        ),
        Ok(violations) => {
            let details = violations
                .iter()
                .take(5)
                .map(|item| format!("{}={} lines", item.path.display(), item.lines))
                .collect::<Vec<_>>()
                .join(", ");
            fail("line-limit", details)
        }
        Err(e) => fail("line-limit", format!("failed to scan Rust files: {e}")),
    }
}

pub fn rust_line_violations(root: &Path, max_lines: usize) -> anyhow::Result<Vec<FileLineCount>> {
    let mut out = Vec::new();
    visit_rust_files(root, root, max_lines, &mut out)?;
    out.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.path.cmp(&b.path)));
    Ok(out)
}

fn visit_rust_files(
    root: &Path,
    dir: &Path,
    max_lines: usize,
    out: &mut Vec<FileLineCount>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if !is_skipped_dir(&path) {
                visit_rust_files(root, &path, max_lines, out)?;
            }
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let lines = std::fs::read_to_string(&path)?.lines().count();
            if lines > max_lines {
                out.push(FileLineCount {
                    path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    lines,
                });
            }
        }
    }
    Ok(())
}

fn is_skipped_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    matches!(name, ".git" | ".dsx" | "target")
}

fn ok(name: &'static str, detail: impl Into<String>) -> Check {
    check(CheckStatus::Ok, name, detail)
}

fn warn(name: &'static str, detail: impl Into<String>) -> Check {
    check(CheckStatus::Warn, name, detail)
}

fn fail(name: &'static str, detail: impl Into<String>) -> Check {
    check(CheckStatus::Fail, name, detail)
}

fn check(status: CheckStatus, name: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status,
        name,
        detail: detail.into(),
    }
}

fn marker(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "[ok]",
        CheckStatus::Warn => "[warn]",
        CheckStatus::Fail => "[fail]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_files_over_line_limit() {
        let root = temp_root("dsx_doctor_line_limit");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/ok.rs"), "x\n".repeat(3)).unwrap();
        std::fs::write(root.join("src/too_long.rs"), "x\n".repeat(4)).unwrap();
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::write(root.join("target/debug/ignored.rs"), "x\n".repeat(10)).unwrap();

        let violations = rust_line_violations(&root, 3).unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, PathBuf::from("src/too_long.rs"));
        assert_eq!(violations[0].lines, 4);

        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
