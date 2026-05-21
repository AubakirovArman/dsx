//! Dry-run model context preview without calling the provider.

use std::path::Path;

pub(crate) struct ContextPreview {
    pub(crate) task: String,
    pub(crate) clean_task: String,
    pub(crate) launch_scope: String,
    pub(crate) active_scope: String,
    pub(crate) narrowed: bool,
    pub(crate) active_exists: bool,
    pub(crate) system_note: String,
    pub(crate) project_context: String,
    pub(crate) task_brief: String,
}

pub async fn run_context_preview(
    project_root: &Path,
    task: &str,
    json: bool,
) -> anyhow::Result<()> {
    let preview = build_context_preview(project_root, task).await?;
    if json {
        println!("{}", preview_json(&preview));
    } else {
        print_preview(&preview);
    }
    Ok(())
}

pub(crate) async fn build_context_preview(
    project_root: &Path,
    task: &str,
) -> anyhow::Result<ContextPreview> {
    let scope = dsx_agent::scope::resolve_task_scope(project_root, task)?;
    let clean_task = dsx_agent::brief::clean_task_input(task);
    let ctx = collect_preview_context(&scope.active_root).await?;
    let project_context = dsx_context::format_context(&ctx);
    let task_brief = dsx_agent::brief::build_task_brief(&clean_task, &scope, &ctx);

    Ok(ContextPreview {
        task: task.into(),
        clean_task,
        launch_scope: scope.launch_root.display().to_string(),
        active_scope: scope.active_root.display().to_string(),
        narrowed: scope.narrowed,
        active_exists: scope.active_root.exists(),
        system_note: scope.system_note(),
        project_context,
        task_brief,
    })
}

async fn collect_preview_context(active_root: &Path) -> anyhow::Result<dsx_context::AgentContext> {
    if active_root.exists() {
        return dsx_context::ContextManager::new()
            .collect(active_root, 250_000)
            .await;
    }

    Ok(dsx_context::AgentContext {
        project_root: active_root.display().to_string(),
        git_status: "active scope does not exist yet".into(),
        git_diff: String::new(),
        file_tree: Vec::new(),
        memories: Vec::new(),
        task_summary: None,
        max_tokens: 250_000,
    })
}

fn print_preview(preview: &ContextPreview) {
    println!("Context preview:");
    println!("  Task: {}", crate::handlers::task_preview(&preview.task));
    println!(
        "  Clean task: {}",
        crate::handlers::task_preview(&preview.clean_task)
    );
    println!("  Launch: {}", preview.launch_scope);
    println!("  Active: {}", preview.active_scope);
    println!(
        "  Status: {}",
        if preview.narrowed { "NARROWED" } else { "WIDE" }
    );
    println!(
        "  Active exists: {}",
        if preview.active_exists { "yes" } else { "no" }
    );
    println!("\nSystem scope note:\n{}\n", preview.system_note);
    println!("Compact task brief:\n{}\n", preview.task_brief);
    println!("Project context:\n{}", preview.project_context);
}

pub(crate) fn preview_json(preview: &ContextPreview) -> serde_json::Value {
    serde_json::json!({
        "task": preview.task,
        "clean_task": preview.clean_task,
        "launch_scope": preview.launch_scope,
        "active_scope": preview.active_scope,
        "narrowed": preview.narrowed,
        "active_exists": preview.active_exists,
        "system_note": preview.system_note,
        "task_brief": preview.task_brief,
        "project_context": preview.project_context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(value["task_brief"].as_str().unwrap().contains("Goal:"));
        assert!(
            value["project_context"]
                .as_str()
                .unwrap()
                .contains("Project:")
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
