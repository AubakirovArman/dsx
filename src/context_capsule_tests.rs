//! Tests for context capsule assembly.

#[cfg(test)]
mod tests {
    use crate::context_capsule::{build_context_capsule, capsule_json};

    #[tokio::test]
    async fn capsule_uses_narrowed_scope_and_folder_notes() {
        let root = temp_root("dsx_context_capsule");
        let child = root.join("1234");
        let other = root.join("other");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(child.join("src")).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(child.join("src").join("main.rs"), "fn main() {}\n").unwrap();

        let capsule = build_context_capsule(&root, "доработай 1234", 5)
            .await
            .unwrap();

        assert!(capsule.narrowed);
        assert_eq!(
            capsule.active_scope,
            child.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(capsule.task_state.goal, "доработай 1234");
        assert!(capsule.task_state.constraints.contains("300 lines"));
        assert!(capsule.folder_notes.iter().any(|note| note.label == "1234"));
        assert!(capsule.metrics.estimated_capsule_tokens > 0);
        let value = capsule_json(&capsule);
        assert_eq!(value["scope_contract"]["status"], "narrowed");
        assert_eq!(value["scope_contract"]["tool_root"], capsule.active_scope);
        assert_eq!(value["scope_contract"]["warning"], "");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn capsule_json_exposes_structured_state() {
        let root = temp_root("dsx_context_capsule_json");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let capsule = build_context_capsule(&root, "build", 2).await.unwrap();
        let value = capsule_json(&capsule);

        assert_eq!(value["task_state"]["goal"], "build");
        assert!(
            value["task_state"]["constraints"]
                .as_str()
                .unwrap()
                .contains("300 lines")
        );
        assert_eq!(value["scope_contract"]["status"], "wide");
        assert_eq!(
            value["scope_contract"]["tool_root"],
            root.canonicalize().unwrap().display().to_string()
        );
        assert!(
            value["scope_contract"]["rule"]
                .as_str()
                .unwrap()
                .contains("active_scope")
        );
        assert!(value["folder_notes"].is_array());
        assert!(
            value["metrics"]["estimated_capsule_tokens"]
                .as_u64()
                .unwrap()
                > 0
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
