//! Actionable remediation for oversized context previews.

use crate::context_preview::ContextPreview;

pub(crate) fn over_budget_error(preview: &ContextPreview) -> String {
    format!(
        "Context preview over request budget: estimated {} tokens, limit {}.\n{}",
        preview.metrics.estimated_request_tokens,
        preview.metrics.max_request_tokens,
        budget_advice(preview)
    )
}

pub(crate) fn budget_advice(preview: &ContextPreview) -> String {
    let mut lines = vec![
        format!("Active scope: {}", preview.active_scope),
        format!(
            "Budget status: {}",
            crate::context_preview::budget_line(preview)
        ),
    ];
    if preview.narrowed {
        lines.push("Keep the task inside this active child scope.".into());
    } else {
        lines.push("Add an explicit child folder like ./1234 before starting the agent.".into());
    }
    lines.push("Run: dsx context --check --require-narrow <task>".into());
    lines.push("Inspect compact state: dsx capsule --limit 4 <task>".into());
    lines.push(
        "Remove generated/vendor directories from the active scope or add ignore rules.".into(),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn advice_mentions_scope_and_safe_commands() {
        let root = temp_root("dsx_context_budget_advice");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();

        let preview = crate::context_preview::build_context_preview(&root, "build 1234")
            .await
            .unwrap();
        let advice = budget_advice(&preview);

        assert!(advice.contains("Active scope:"));
        assert!(advice.contains("dsx context --check --require-narrow"));
        assert!(advice.contains("dsx capsule --limit 4"));

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
