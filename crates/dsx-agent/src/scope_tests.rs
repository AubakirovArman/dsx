//! Tests for launch workspace to active task scope resolution.

#[cfg(test)]
mod tests {
    use crate::scope::{ensure_active_root, resolve_task_scope};
    use std::path::PathBuf;

    #[test]
    fn narrows_to_absolute_subdirectory() {
        let root = temp_root("dsx_scope_abs");
        let target = root.join("sites/1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let task = format!("создай проект только в {}", target.display());
        let scope = resolve_task_scope(&root, &task).unwrap();

        assert_eq!(scope.active_root, target.canonicalize().unwrap());
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn narrows_to_dot_relative_subdirectory() {
        let root = temp_root("dsx_scope_dot_relative");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let scope = resolve_task_scope(&root, "используй только ./1234").unwrap();

        assert_eq!(scope.active_root, target.canonicalize().unwrap());
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn narrows_to_bare_child_directory_when_scope_requested() {
        let root = temp_root("dsx_scope_bare_child");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let scope = resolve_task_scope(&root, "используй папку 1234 только").unwrap();

        assert_eq!(scope.active_root, target.canonicalize().unwrap());
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn narrows_to_missing_bare_directory_when_creation_requested() {
        let root = temp_root("dsx_scope_missing_bare");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let scope = resolve_task_scope(&root, "создай проект 1234").unwrap();

        assert_eq!(scope.active_root, target);
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_bare_directory_ignores_scope_words() {
        let root = temp_root("dsx_scope_missing_bare_words");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let scope = resolve_task_scope(&root, "создай проект в папке 1234").unwrap();

        assert_eq!(scope.active_root, target);
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn narrows_existing_bare_child_even_without_scope_hint() {
        let root = temp_root("dsx_scope_existing_bare");
        let target = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let scope = resolve_task_scope(&root, "почини 1234").unwrap();

        assert_eq!(scope.active_root, target.canonicalize().unwrap());
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn does_not_narrow_missing_bare_word_without_scope_hint() {
        let root = temp_root("dsx_scope_missing_no_hint");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let scope = resolve_task_scope(&root, "почини 1234").unwrap();

        assert_eq!(scope.active_root, root.canonicalize().unwrap());
        assert!(!scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ignores_paths_outside_launch_workspace() {
        let root = temp_root("dsx_scope_inside");
        let outside = temp_root("dsx_scope_outside");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let task = format!("используй {}", outside.display());
        let scope = resolve_task_scope(&root, &task).unwrap();

        assert_eq!(scope.active_root, root.canonicalize().unwrap());
        assert!(!scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn narrows_to_missing_directory_inside_launch_workspace() {
        let root = temp_root("dsx_scope_missing_dir");
        let target = root.join("sites/1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sites")).unwrap();

        let task = format!("создай проект только в {}", target.display());
        let scope = resolve_task_scope(&root, &task).unwrap();

        assert_eq!(scope.active_root, target);
        assert!(scope.narrowed);
        assert!(!scope.active_root.exists());
        ensure_active_root(&scope).unwrap();
        assert!(scope.active_root.is_dir());

        let _ = std::fs::remove_dir_all(&root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
