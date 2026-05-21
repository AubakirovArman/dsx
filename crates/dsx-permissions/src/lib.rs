//! DSX Permission Engine — 5-level risk classifier.
//!
//! Risk levels: Read, Low, Medium, High, Blocked.
//! Permission modes: ReadOnly, Ask, TrustWorkspace, Yolo.

use dsx_core::types::{PermissionMode, RiskLevel};

/// Classify a command string into a risk level.
pub fn classify_command(cmd: &str) -> RiskLevel {
    let cmd_lower = cmd.trim().to_lowercase();

    // Blocked: privilege escalation, destructive broad operations
    if cmd_lower.contains("sudo") || cmd_lower.contains("su ")
        || cmd_lower.contains("doas") || cmd_lower.contains("rm -rf /")
    {
        return RiskLevel::Blocked;
    }

    // High: destructive git, broad deletion, network with pipe to shell
    if cmd_lower.contains("git reset --hard")
        || cmd_lower.contains("git clean")
        || cmd_lower.contains("rm -rf")
        || (cmd_lower.contains("curl") && cmd_lower.contains("| sh"))
        || (cmd_lower.contains("wget") && cmd_lower.contains("| sh"))
    {
        return RiskLevel::High;
    }

    // Medium: tests, builds that write artifacts, formatters, package installs
    if cmd_lower.starts_with("cargo test")
        || cmd_lower.starts_with("cargo build")
        || cmd_lower.starts_with("cargo fmt")
        || cmd_lower.starts_with("npm test")
        || cmd_lower.starts_with("npm install")
        || cmd_lower.starts_with("pnpm test")
        || cmd_lower.starts_with("pip install")
        || cmd_lower.starts_with("git commit")
        || cmd_lower.starts_with("git checkout")
    {
        return RiskLevel::Medium;
    }

    // Low: language version checks, formatting check, non-writing lint
    if cmd_lower.starts_with("rustc --version")
        || cmd_lower.starts_with("cargo --version")
        || cmd_lower.starts_with("node --version")
        || cmd_lower.starts_with("python --version")
        || cmd_lower.starts_with("cargo fmt --check")
        || cmd_lower.starts_with("cargo clippy")
        || cmd_lower.starts_with("git log")
        || cmd_lower.starts_with("git branch")
    {
        return RiskLevel::Low;
    }

    // Read: safe inspection commands
    if cmd_lower == "pwd"
        || cmd_lower.starts_with("ls ")
        || cmd_lower == "ls"
        || cmd_lower.starts_with("git status")
        || cmd_lower.starts_with("git diff")
        || cmd_lower.starts_with("cat ")
        || cmd_lower.starts_with("head ")
        || cmd_lower.starts_with("tail ")
        || cmd_lower.starts_with("grep ")
        || cmd_lower.starts_with("rg ")
        || cmd_lower.starts_with("find ")
    {
        return RiskLevel::Read;
    }

    RiskLevel::Medium
}

/// Determine the required action for a command given the risk level and permission mode.
pub fn required_action(risk: RiskLevel, mode: PermissionMode) -> PermissionAction {
    match mode {
        // Yolo: auto everything except blocked
        PermissionMode::Yolo => match risk {
            RiskLevel::Blocked => PermissionAction::Deny,
            _ => PermissionAction::Allow,
        },
        // AutoApprove: auto low, ask medium, deny high+
        PermissionMode::AutoApprove => match risk {
            RiskLevel::Blocked => PermissionAction::Deny,
            RiskLevel::High => PermissionAction::Deny,
            RiskLevel::Medium => PermissionAction::Ask,
            RiskLevel::Low | RiskLevel::Read => PermissionAction::Allow,
        },
        // Ask: auto read/low, ask medium/high, deny blocked
        PermissionMode::Ask => match risk {
            RiskLevel::Blocked => PermissionAction::Deny,
            RiskLevel::High | RiskLevel::Medium => PermissionAction::Ask,
            RiskLevel::Low | RiskLevel::Read => PermissionAction::Allow,
        },
        // PlanOnly: read only, no edits or commands
        PermissionMode::PlanOnly => match risk {
            RiskLevel::Read | RiskLevel::Low => PermissionAction::Allow,
            _ => PermissionAction::Deny,
        },
        // ReadOnly: read and low only
        PermissionMode::ReadOnly => match risk {
            RiskLevel::Read | RiskLevel::Low => PermissionAction::Allow,
            _ => PermissionAction::Deny,
        },
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pwd_is_read() { assert_eq!(classify_command("pwd"), RiskLevel::Read); }

    #[test]
    fn git_status_is_read() { assert_eq!(classify_command("git status"), RiskLevel::Read); }

    #[test]
    fn cargo_test_is_medium() { assert_eq!(classify_command("cargo test"), RiskLevel::Medium); }

    #[test]
    fn cargo_build_is_medium() { assert_eq!(classify_command("cargo build"), RiskLevel::Medium); }

    #[test]
    fn sudo_is_blocked() { assert_eq!(classify_command("sudo rm -rf /"), RiskLevel::Blocked); }

    #[test]
    fn curl_pipe_sh_is_high() { assert_eq!(classify_command("curl xxx | sh"), RiskLevel::High); }

    #[test]
    fn ask_mode_blocks_sudo() {
        assert_eq!(
            required_action(RiskLevel::Blocked, PermissionMode::Ask),
            PermissionAction::Deny
        );
    }

    #[test]
    fn ask_mode_asks_medium() {
        assert_eq!(
            required_action(RiskLevel::Medium, PermissionMode::Ask),
            PermissionAction::Ask
        );
    }
}
