//! Tests for agent-start preflight diagnostics.

#[cfg(test)]
mod tests {
    use crate::agent_preflight::build_agent_preflight;

    #[test]
    fn preflight_blocks_wide_container_workspace() {
        let root = temp_root("dsx_preflight_block");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234")).unwrap();
        std::fs::create_dir_all(root.join("other")).unwrap();

        let preflight = build_agent_preflight(&root, "доработай проект", false);

        assert!(!preflight.allowed());
        assert!(!preflight.narrowed);
        assert!(
            preflight
                .reason
                .contains("Wide container workspace blocked")
        );
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
        assert_eq!(
            preflight.active,
            target.canonicalize().unwrap().display().to_string()
        );
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
