//! Tests for CLI command handlers.

#[cfg(test)]
mod tests {
    use crate::handlers::prepare_cli_agent_scope;

    #[test]
    fn cli_agent_scope_blocks_wide_container_workspace() {
        let root = temp_root("dsx_cli_scope_block");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let result = prepare_cli_agent_scope(&root, "доработай проект", false);

        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("Wide container workspace blocked")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cli_agent_scope_allows_explicit_policy_override() {
        let root = temp_root("dsx_cli_scope_allow_wide");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let scope = prepare_cli_agent_scope(&root, "доработай проект", true).unwrap();

        assert!(!scope.narrowed);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cli_agent_scope_allows_explicit_child_folder() {
        let root = temp_root("dsx_cli_scope_narrow");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let scope = prepare_cli_agent_scope(&root, "доработай 1234", false).unwrap();

        assert!(scope.narrowed);
        assert_eq!(scope.active_root, target.canonicalize().unwrap());
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
