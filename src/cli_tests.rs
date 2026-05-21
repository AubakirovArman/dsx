//! Tests for CLI parser options.

#[cfg(test)]
mod tests {
    use crate::cli::{CliArgs, Command, WorkspaceAction};
    use clap::Parser;

    #[test]
    fn parses_allow_wide_scope_global_flag() {
        let cli =
            CliArgs::try_parse_from(["dsx", "--allow-wide-scope", "edit", "доработай", "проект"])
                .unwrap();

        assert!(cli.allow_wide_scope);
    }

    #[test]
    fn parses_preflight_json_check_flags() {
        let cli =
            CliArgs::try_parse_from(["dsx", "preflight", "--json", "--check", "почини", "1234"])
                .unwrap();

        let Some(Command::Preflight { task, json, check }) = cli.command else {
            panic!("expected preflight command");
        };
        assert_eq!(task, vec!["почини".to_string(), "1234".to_string()]);
        assert!(json);
        assert!(check);
    }

    #[test]
    fn parses_capsule_json_limit_flags() {
        let cli =
            CliArgs::try_parse_from(["dsx", "capsule", "--json", "--limit", "3", "почини", "1234"])
                .unwrap();

        let Some(Command::Capsule { task, limit, json }) = cli.command else {
            panic!("expected capsule command");
        };
        assert_eq!(task, vec!["почини".to_string(), "1234".to_string()]);
        assert_eq!(limit, 3);
        assert!(json);
    }

    #[test]
    fn parses_workspace_audit_flags() {
        let cli = CliArgs::try_parse_from([
            "dsx",
            "workspace",
            "audit",
            "--json",
            "--all",
            "--limit",
            "4",
        ])
        .unwrap();

        let Some(Command::Workspace {
            action: Some(WorkspaceAction::Audit { limit, all, json }),
        }) = cli.command
        else {
            panic!("expected workspace audit command");
        };
        assert_eq!(limit, 4);
        assert!(all);
        assert!(json);
    }
}
