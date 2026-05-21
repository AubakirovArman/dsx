//! Scope preflight for shell commands.

use std::path::{Path, PathBuf};

pub fn validate_command_scope(command: &str, workspace: &Path) -> Result<(), String> {
    let workspace = workspace
        .canonicalize()
        .map_err(|e| format!("cannot resolve active scope: {e}"))?;
    let tokens = shell_tokens(command);

    validate_directory_changes(&tokens, &workspace)?;
    for token in tokens {
        for fragment in path_fragments(&token) {
            validate_path_fragment(&workspace, &fragment)?;
        }
    }

    Ok(())
}

fn validate_directory_changes(tokens: &[String], workspace: &Path) -> Result<(), String> {
    for pair in tokens.windows(2) {
        let command = pair[0].as_str();
        let target = pair[1].as_str();
        if !matches!(command, "cd" | "pushd") || target.starts_with('-') {
            continue;
        }
        if target.contains('$') {
            return Err(format!("command path leaves active scope: {target}"));
        }
        validate_path_fragment(workspace, target)?;
    }
    Ok(())
}

fn validate_path_fragment(workspace: &Path, raw: &str) -> Result<(), String> {
    if raw == "~" || raw.starts_with("~/") {
        return Err(format!("command path leaves active scope: {raw}"));
    }

    let path = PathBuf::from(raw);
    let candidate = if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    };
    let anchor = existing_anchor(&candidate)
        .map_err(|e| format!("cannot validate command path {raw}: {e}"))?;

    if anchor.starts_with(workspace) {
        Ok(())
    } else {
        Err(format!("command path leaves active scope: {raw}"))
    }
}

fn existing_anchor(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }

    let mut current = path;
    while !current.exists() {
        current = current
            .parent()
            .ok_or_else(|| anyhow::anyhow!("path has no existing parent"))?;
    }
    Ok(current.canonicalize()?)
}

fn path_fragments(token: &str) -> Vec<String> {
    token
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')'))
        .filter_map(|part| {
            let part = part.trim_matches(trim_shell_punctuation);
            if part.is_empty() {
                return None;
            }
            if looks_like_scoped_path(part) {
                return Some(part.to_string());
            }
            part.split_once('=')
                .map(|(_, value)| value.trim_matches(trim_shell_punctuation))
                .filter(|value| looks_like_scoped_path(value))
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn looks_like_scoped_path(value: &str) -> bool {
    value == "~"
        || value.starts_with("~/")
        || value.starts_with('/')
        || value == ".."
        || value.starts_with("../")
        || value.contains("/../")
        || value.ends_with("/..")
}

fn trim_shell_punctuation(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '`' | ',' | ':' | ';' | ')' | '(' | ']' | '[' | '}' | '{'
    )
}

fn shell_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in command.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() || matches!(ch, ';' | '&' | '|' | '<' | '>') {
            push_token(&mut tokens, &mut current);
        } else {
            current.push(ch);
        }
    }
    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    current.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_relative_commands_inside_scope() {
        let root = temp_root("dsx_cmd_scope_allow");
        std::fs::create_dir_all(root.join("src")).unwrap();

        validate_command_scope("find src -type f && cargo test --workspace", &root).unwrap();

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blocks_parent_directory_escape() {
        let root = temp_root("dsx_cmd_scope_parent");
        std::fs::create_dir_all(&root).unwrap();

        let err = validate_command_scope("cd .. && find .", &root).unwrap_err();

        assert!(err.contains("leaves active scope"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blocks_absolute_path_outside_scope() {
        let root = temp_root("dsx_cmd_scope_abs");
        let outside = temp_root("dsx_cmd_scope_outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let command = format!("find {}", outside.display());
        let err = validate_command_scope(&command, &root).unwrap_err();

        assert!(err.contains("leaves active scope"));
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    #[test]
    fn allows_absolute_path_inside_scope() {
        let root = temp_root("dsx_cmd_scope_abs_inside");
        let inside = root.join("app");
        std::fs::create_dir_all(&inside).unwrap();

        let command = format!("ls {}", inside.display());
        validate_command_scope(&command, &root).unwrap();

        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
