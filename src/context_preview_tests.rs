//! Tests for dry-run context preview assembly.

#[cfg(test)]
mod tests {
    use crate::context_preview::{
        build_context_preview, enforce_narrow_scope, enforce_request_budget, preview_json,
    };

    #[tokio::test]
    async fn context_preview_uses_narrowed_existing_scope() {
        let root = temp_root("dsx_context_preview_existing");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(child.join("Cargo.toml"), "[package]\n").unwrap();

        let preview = build_context_preview(&root, "почини 1234").await.unwrap();

        assert!(preview.narrowed);
        assert!(preview.active_exists);
        assert_eq!(
            preview.active_scope,
            child.canonicalize().unwrap().display().to_string()
        );
        assert!(preview.project_context.contains("Cargo.toml"));
        assert!(preview.task_brief.contains("Active scope:"));
        let value = preview_json(&preview);
        assert_eq!(value["scope_contract"]["status"], "narrowed");
        assert_eq!(value["scope_contract"]["tool_root"], preview.active_scope);
        assert_eq!(value["scope_contract"]["warning"], "");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_does_not_create_missing_scope() {
        let root = temp_root("dsx_context_preview_missing");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let preview = build_context_preview(&root, "создай проект 1234")
            .await
            .unwrap();

        assert!(preview.narrowed);
        assert!(!preview.active_exists);
        assert!(!child.exists());
        assert!(preview.project_context.contains("does not exist yet"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_json_contains_prompt_parts() {
        let root = temp_root("dsx_context_preview_json");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let preview = build_context_preview(&root, "build").await.unwrap();
        let value = preview_json(&preview);

        assert_eq!(
            value["active_scope"],
            root.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(value["scope_contract"]["status"], "wide");
        assert_eq!(
            value["scope_contract"]["tool_root"],
            root.canonicalize().unwrap().display().to_string()
        );
        assert!(
            value["scope_contract"]["warning"]
                .as_str()
                .unwrap()
                .contains("workspace-wide")
        );
        assert!(value["task_brief"].as_str().unwrap().contains("Goal:"));
        assert!(
            value["context_capsule"]
                .as_str()
                .unwrap()
                .contains("previous chat history")
        );
        assert!(
            value["budget_advice"]
                .as_str()
                .unwrap()
                .contains("dsx context --check")
        );
        assert!(
            value["project_context"]
                .as_str()
                .unwrap()
                .contains("Project:")
        );
        assert_eq!(value["metrics"]["request_budget_status"], "ok");
        assert!(
            value["metrics"]["estimated_request_tokens"]
                .as_u64()
                .unwrap()
                > 0
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_includes_project_instructions_in_metrics() {
        let root = temp_root("dsx_context_preview_instructions");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("AGENTS.md"), "Always stay scoped.\n").unwrap();

        let preview = build_context_preview(&root, "build").await.unwrap();

        assert!(preview.project_instructions.is_some());
        assert!(preview.metrics.project_instructions_chars > 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_skips_generated_file_tree_dirs() {
        let root = temp_root("dsx_context_preview_generated");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(child.join("src")).unwrap();
        std::fs::create_dir_all(child.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(child.join("target/debug")).unwrap();
        std::fs::create_dir_all(child.join("dist")).unwrap();

        let preview = build_context_preview(&root, "build 1234").await.unwrap();

        assert!(preview.project_context.contains("src/"));
        assert!(!preview.project_context.contains("node_modules/"));
        assert!(!preview.project_context.contains("target/"));
        assert!(!preview.project_context.contains("dist/"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_check_rejects_over_budget() {
        let root = temp_root("dsx_context_preview_check");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let mut preview = build_context_preview(&root, "build").await.unwrap();
        preview.metrics.max_request_tokens = preview.metrics.estimated_request_tokens - 1;

        let err = enforce_request_budget(&preview).unwrap_err();

        assert!(err.to_string().contains("over request budget"));
        assert!(err.to_string().contains("dsx capsule --limit 4"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_require_narrow_rejects_wide_scope() {
        let root = temp_root("dsx_context_preview_require_narrow");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let preview = build_context_preview(&root, "build").await.unwrap();
        let err = enforce_narrow_scope(&preview).unwrap_err();

        assert!(err.to_string().contains("launch workspace"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn context_preview_require_narrow_allows_child_scope() {
        let root = temp_root("dsx_context_preview_require_narrow_child");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();

        let preview = build_context_preview(&root, "build 1234").await.unwrap();

        enforce_narrow_scope(&preview).unwrap();
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
