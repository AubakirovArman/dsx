//! DSX Git — thin wrapper around git CLI for status, diff, checkpoints.

use std::path::Path;
use std::process::Command;

/// Run a git command in the given directory.
fn git(args: &[&str], cwd: &Path) -> anyhow::Result<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git error: {err}")
    }
}

pub fn status(cwd: &Path) -> anyhow::Result<String> {
    git(&["status", "--porcelain=v1", "-b"], cwd)
}

pub fn diff(cwd: &Path) -> anyhow::Result<String> {
    git(&["diff"], cwd)
}

pub fn diff_staged(cwd: &Path) -> anyhow::Result<String> {
    git(&["diff", "--cached"], cwd)
}

/// Create a git commit checkpoint before an edit.
pub fn checkpoint(label: &str, cwd: &Path) -> anyhow::Result<()> {
    git(&["add", "-A"], cwd)?;
    let msg = format!("DSX checkpoint: {label}");
    git(&["commit", "-m", &msg, "--allow-empty"], cwd)?;
    Ok(())
}

/// Restore the working tree to the latest DSX checkpoint.
pub fn rollback(cwd: &Path) -> anyhow::Result<String> {
    let last_msg = git(&["log", "-1", "--pretty=%B"], cwd)?;
    if last_msg.trim().starts_with("DSX checkpoint:") {
        git(&["reset", "--hard", "HEAD"], cwd)?;
        Ok(format!(
            "Successfully restored checkpoint: {}",
            last_msg.trim()
        ))
    } else {
        anyhow::bail!("No active DSX checkpoint found at HEAD. Cannot undo.")
    }
}

/// Check if the working tree is dirty.
pub fn is_dirty(cwd: &Path) -> anyhow::Result<bool> {
    let out = git(&["status", "--porcelain"], cwd)?;
    Ok(!out.trim().is_empty())
}

/// Get current branch name.
pub fn current_branch(cwd: &Path) -> anyhow::Result<String> {
    let out = git(&["branch", "--show-current"], cwd)?;
    Ok(out.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_and_rollback() {
        let tmp = std::env::temp_dir().join("dsx_git_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Init git repo
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&tmp)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(&tmp)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&tmp)
            .output();

        // Initial commit
        std::fs::write(tmp.join("a.txt"), "initial").unwrap();
        git(&["add", "-A"], &tmp).unwrap();
        git(&["commit", "-m", "initial commit"], &tmp).unwrap();

        // User dirty state and checkpoint
        std::fs::write(tmp.join("a.txt"), "user changes").unwrap();
        checkpoint("edit-1", &tmp).unwrap();

        // Simulate agent edit after checkpoint.
        std::fs::write(tmp.join("a.txt"), "agent changes").unwrap();

        // Rollback
        let res = rollback(&tmp).unwrap();
        assert!(res.contains("edit-1"));

        let final_content = std::fs::read_to_string(tmp.join("a.txt")).unwrap();
        assert_eq!(final_content, "user changes");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
