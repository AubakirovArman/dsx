//! Tests for agent-start preflight diagnostics.

#[cfg(test)]
mod tests {
    use crate::agent_preflight::{build_agent_preflight, prepare_agent_start_scope, render_text};

    #[test]
    fn preflight_blocks_wide_container_workspace() {
        let root = temp_root("dsx_preflight_block");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let preflight = build_agent_preflight(&root, "доработай проект", false);

        assert!(!preflight.allowed());
        assert!(!preflight.narrowed);
        assert_eq!(preflight.policy_source, "container_guard");
        assert!(
            preflight
                .reason
                .contains("Wide container workspace blocked")
        );
        assert!(render_text(&preflight).contains("Decision: BLOCKED"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_allows_wide_container_with_policy() {
        let root = temp_root("dsx_preflight_allow_wide");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let preflight = build_agent_preflight(&root, "доработай проект", true);

        assert!(preflight.allowed());
        assert!(preflight.allow_wide_scope);
        assert_eq!(preflight.policy_source, "allow_wide_policy");
        assert!(preflight.reason.contains("explicit CLI/config policy"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_allows_explicit_child_scope() {
        let root = temp_root("dsx_preflight_child");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let preflight = build_agent_preflight(&root, "доработай 1234", false);

        assert!(preflight.allowed());
        assert!(preflight.narrowed);
        assert_eq!(preflight.policy_source, "task_scope");
        assert_eq!(
            preflight.active,
            target.canonicalize().unwrap().display().to_string()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_agent_start_scope_returns_blocking_preflight_error() {
        let root = temp_root("dsx_preflight_start_scope_block");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let result = prepare_agent_start_scope(&root, "доработай проект", false);

        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("agent preflight blocked")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn render_text_renders_same_preflight_report() {
        let root = temp_root("dsx_preflight_block_text");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();

        let preflight = build_agent_preflight(&root, "доработай проект", false);
        let text = render_text(&preflight);

        assert!(text.contains("Agent preflight"));
        assert!(text.contains("Policy source: container_guard"));
        assert!(text.contains("Decision: BLOCKED"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_reports_explicit_wide_intent_source() {
        let root = temp_root("dsx_preflight_wide_intent");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();

        let preflight = build_agent_preflight(&root, "проверь весь воркспейс", false);

        assert!(preflight.allowed());
        assert_eq!(preflight.policy_source, "task_wide_intent");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_agent_start_scope_allows_policy_override() {
        let root = temp_root("dsx_preflight_start_scope_allow");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let scope = prepare_agent_start_scope(&root, "доработай проект", true).unwrap();

        assert!(!scope.narrowed);
        assert_eq!(scope.active_root, root.canonicalize().unwrap());
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
