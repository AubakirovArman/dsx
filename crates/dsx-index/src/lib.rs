//! DSX Index — codebase indexing: file discovery, symbol extraction, search.
//!
//! MVP: file discovery + metadata. Full tree-sitter/tantivy in v1.

use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String, // "function", "struct", "class"
    pub start_line: u32,
    pub end_line: u32,
    pub signature: String,
}

/// Scan a project and return a ranked list of important files.
pub fn scan_project(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(root).build() {
        let entry = entry?;
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let path = entry.path().strip_prefix(root).unwrap_or(entry.path());
            files.push(path.display().to_string());
        }
    }
    // Sort: config/root files first, then source, then rest
    files.sort_by(|a, b| {
        let rank_a = file_rank(a);
        let rank_b = file_rank(b);
        rank_a.cmp(&rank_b).then_with(|| a.cmp(b))
    });
    Ok(files)
}

fn file_rank(path: &str) -> u8 {
    if path == "Cargo.toml" || path == "package.json" || path == "go.mod" || path == "pyproject.toml" {
        0
    } else if path == "README.md" || path == "Makefile" {
        1
    } else if path.starts_with("src/") || path.starts_with("crates/") || path.starts_with("lib/") {
        2
    } else if path.starts_with("tests/") || path.ends_with("_test.rs") || path.ends_with(".test.ts") {
        3
    } else {
        5
    }
}

/// Detect project language based on configuration files.
pub fn detect_language(root: &Path) -> Vec<String> {
    let mut langs = Vec::new();
    if root.join("Cargo.toml").exists() { langs.push("rust".into()); }
    if root.join("package.json").exists() { langs.push("typescript".into()); }
    if root.join("go.mod").exists() { langs.push("go".into()); }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() { langs.push("python".into()); }
    if root.join("pom.xml").exists() { langs.push("java".into()); }
    langs
}

/// Parse a source file and extract structural symbols.
pub fn extract_symbols(content: &str, file_ext: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    // We only index supported languages
    if file_ext != "rs" && file_ext != "ts" && file_ext != "js" && file_ext != "py" {
        return symbols;
    }

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        
        if file_ext == "rs" {
            // Match Rust functions
            if line.contains("fn ") && (line.starts_with("fn ") || line.starts_with("pub ") || line.starts_with("async ") || line.starts_with("pub async ")) {
                if let Some(fn_name) = extract_rust_name(line, "fn ") {
                    let start = i as u32 + 1;
                    let end = find_closing_brace(&lines, i) as u32 + 1;
                    symbols.push(Symbol {
                        name: fn_name,
                        kind: "function".into(),
                        start_line: start,
                        end_line: end,
                        signature: line.to_string(),
                    });
                }
            }
            // Match Rust structs
            else if line.contains("struct ") && (line.starts_with("struct ") || line.starts_with("pub ")) {
                if let Some(struct_name) = extract_rust_name(line, "struct ") {
                    let start = i as u32 + 1;
                    let end = find_closing_brace(&lines, i) as u32 + 1;
                    symbols.push(Symbol {
                        name: struct_name,
                        kind: "struct".into(),
                        start_line: start,
                        end_line: end,
                        signature: line.to_string(),
                    });
                }
            }
        } else if file_ext == "py" {
            // Match Python functions/methods
            if line.starts_with("def ") {
                let signature = line.trim_end_matches(':').to_string();
                let name = line["def ".len()..].split('(').next().unwrap_or("").trim().to_string();
                if !name.is_empty() {
                    let start = i as u32 + 1;
                    let end = find_python_block_end(&lines, i) as u32 + 1;
                    symbols.push(Symbol {
                        name,
                        kind: "function".into(),
                        start_line: start,
                        end_line: end,
                        signature,
                    });
                }
            }
            // Match Python classes
            else if line.starts_with("class ") {
                let signature = line.trim_end_matches(':').to_string();
                let name = line["class ".len()..].split(':').next().unwrap_or("").split('(').next().unwrap_or("").trim().to_string();
                if !name.is_empty() {
                    let start = i as u32 + 1;
                    let end = find_python_block_end(&lines, i) as u32 + 1;
                    symbols.push(Symbol {
                        name,
                        kind: "class".into(),
                        start_line: start,
                        end_line: end,
                        signature,
                    });
                }
            }
        } else if file_ext == "ts" || file_ext == "js" {
            // Match JS/TS classes and functions
            if line.contains("class ") && (line.starts_with("class ") || line.starts_with("export ")) {
                if let Some(class_name) = extract_js_name(line, "class ") {
                    let start = i as u32 + 1;
                    let end = find_closing_brace(&lines, i) as u32 + 1;
                    symbols.push(Symbol {
                        name: class_name,
                        kind: "class".into(),
                        start_line: start,
                        end_line: end,
                        signature: line.to_string(),
                    });
                }
            } else if line.contains("function ") && (line.starts_with("function ") || line.starts_with("export ") || line.starts_with("async ")) {
                if let Some(fn_name) = extract_js_name(line, "function ") {
                    let start = i as u32 + 1;
                    let end = find_closing_brace(&lines, i) as u32 + 1;
                    symbols.push(Symbol {
                        name: fn_name,
                        kind: "function".into(),
                        start_line: start,
                        end_line: end,
                        signature: line.to_string(),
                    });
                }
            }
        }
        
        i += 1;
    }
    
    symbols
}

fn extract_rust_name(line: &str, keyword: &str) -> Option<String> {
    if let Some(idx) = line.find(keyword) {
        let remainder = &line[idx + keyword.len()..];
        let name = remainder.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

fn extract_js_name(line: &str, keyword: &str) -> Option<String> {
    if let Some(idx) = line.find(keyword) {
        let remainder = &line[idx + keyword.len()..];
        let name = remainder.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

fn find_closing_brace(lines: &[&str], start_idx: usize) -> usize {
    let mut brace_count = 0;
    let mut found_first = false;
    for idx in start_idx..lines.len() {
        let line = lines[idx];
        for ch in line.chars() {
            if ch == '{' {
                brace_count += 1;
                found_first = true;
            } else if ch == '}' {
                brace_count -= 1;
            }
        }
        if found_first && brace_count <= 0 {
            return idx;
        }
    }
    lines.len() - 1
}

fn find_python_block_end(lines: &[&str], start_idx: usize) -> usize {
    if start_idx + 1 >= lines.len() {
        return start_idx;
    }
    let first_line_indent = lines[start_idx].chars().take_while(|c| c.is_whitespace()).count();
    
    for idx in start_idx + 1..lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if indent <= first_line_indent && !line.trim().starts_with('#') {
            return idx - 1;
        }
    }
    lines.len() - 1
}

/// Build a codebase symbol index and save it to the SQLite database.
pub async fn build_symbol_index(root: &Path, pool: &sqlx::SqlitePool) -> anyhow::Result<usize> {
    // 1. Scan files
    let files = scan_project(root)?;
    let mut count = 0;
    
    // Clear old symbols
    sqlx::query("DELETE FROM symbols WHERE project_root = ?")
        .bind(root.display().to_string())
        .execute(pool)
        .await?;
        
    for file_path_str in files {
        let full_path = root.join(&file_path_str);
        if let Some(ext) = full_path.extension().and_then(|s| s.to_str()) {
            if ext == "rs" || ext == "ts" || ext == "js" || ext == "py" {
                if let Ok(content) = tokio::fs::read_to_string(&full_path).await {
                    let symbols = extract_symbols(&content, ext);
                    for s in symbols {
                        let id = uuid::Uuid::new_v4().to_string();
                        sqlx::query(
                            "INSERT INTO symbols (id, project_root, path, file_hash, language, kind, name, start_line, end_line, signature) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                        )
                        .bind(&id)
                        .bind(root.display().to_string())
                        .bind(&file_path_str)
                        .bind("") // file hash can be empty for MVP
                        .bind(ext)
                        .bind(&s.kind)
                        .bind(&s.name)
                        .bind(s.start_line as i32)
                        .bind(s.end_line as i32)
                        .bind(&s.signature)
                        .execute(pool)
                        .await?;
                        count += 1;
                    }
                }
            }
        }
    }
    
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_symbols() {
        let code = r#"
            pub struct AppState {
                pub count: usize,
            }

            impl AppState {
                pub fn new() -> Self {
                    AppState { count: 0 }
                }
            }
        "#;
        let symbols = extract_symbols(code, "rs");
        assert_eq!(symbols.len(), 2);
        
        let s0 = &symbols[0];
        assert_eq!(s0.name, "AppState");
        assert_eq!(s0.kind, "struct");
        assert_eq!(s0.start_line, 2);
        assert_eq!(s0.end_line, 4);

        let s1 = &symbols[1];
        assert_eq!(s1.name, "new");
        assert_eq!(s1.kind, "function");
        assert_eq!(s1.start_line, 7);
        assert_eq!(s1.end_line, 9);
    }
}
