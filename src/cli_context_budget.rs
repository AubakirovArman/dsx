//! CLI context-capsule budget preflight before agent model calls.

use std::path::Path;

pub async fn preflight_cli_context_budget(project_root: &Path, task: &str) -> anyhow::Result<()> {
    let preview = crate::context_preview::build_context_preview(project_root, task).await?;
    println!(
        "Context budget preflight: {}",
        crate::context_preview::budget_line(&preview)
    );
    crate::context_preview::enforce_request_budget(&preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cli_context_budget_uses_context_preview_capsule_metrics() {
        let root = temp_root("dsx_cli_context_budget");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(child.join("src")).unwrap();
        std::fs::write(child.join("src").join("main.rs"), "fn main() {}\n").unwrap();

        let preview = crate::context_preview::build_context_preview(&root, "build 1234")
            .await
            .unwrap();
        let line = crate::context_preview::budget_line(&preview);

        assert!(line.contains("capsule request"));
        assert!(line.contains("(ok)"));
        preflight_cli_context_budget(&root, "build 1234")
            .await
            .unwrap();

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
