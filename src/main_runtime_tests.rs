//! Tests for main runtime helper behavior.

#[cfg(test)]
mod tests {
    use crate::cli::CliArgs;
    use crate::main_runtime::{api_key, api_key_file};
    use clap::Parser;

    #[test]
    fn api_key_falls_back_to_parent_deepseek_file() {
        let root = temp_root("dsx_key_file");
        let project = root.join("sites").join("dst");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(root.join(".deepseek"), "file-key\n").unwrap();

        let key = api_key_file(&project);

        assert_eq!(key.as_deref(), Some("file-key"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn api_key_prefers_cli_value() {
        let root = temp_root("dsx_key_cli");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".deepseek"), "file-key\n").unwrap();
        let cli = CliArgs::try_parse_from(["dsx", "--api-key", "cli-key"]).unwrap();

        let key = api_key(&cli, &dsx_config::AppConfig::default(), &root);

        assert_eq!(key.as_deref(), Some("cli-key"));
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
