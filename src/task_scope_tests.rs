//! Tests for app-level task scope preview data.

#[cfg(test)]
mod tests {
    use crate::task_scope::resolve_task_scope;

    #[test]
    fn scope_preview_reports_narrowed_subfolder() {
        let root = temp_root("dsx_task_scope_narrow");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let scope = resolve_task_scope(&root, "используй папку 1234 только");

        assert!(scope.narrowed);
        assert_eq!(scope.launch_label, root.display().to_string());
        assert_eq!(
            scope.active_label,
            target.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(scope.active_root, target.canonicalize().unwrap());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scope_preview_reports_wide_workspace() {
        let root = temp_root("dsx_task_scope_wide");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let scope = resolve_task_scope(&root, "почини 1234");

        assert!(!scope.narrowed);
        assert_eq!(scope.active_root, root);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scope_preview_locks_existing_bare_child() {
        let root = temp_root("dsx_task_scope_existing_child");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let scope = resolve_task_scope(&root, "почини 1234");

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
