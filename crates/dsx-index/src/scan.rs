//! File discovery and full-text file search.

use crate::types::FileMatch;
use std::path::Path;

pub fn scan_project(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(root).build() {
        let entry = entry?;
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let path = entry.path().strip_prefix(root).unwrap_or(entry.path());
            files.push(path.display().to_string());
        }
    }
    files.sort_by(|a, b| file_rank(a).cmp(&file_rank(b)).then_with(|| a.cmp(b)));
    Ok(files)
}

pub fn detect_language(root: &Path) -> Vec<String> {
    let mut langs = Vec::new();
    push_if(&mut langs, root.join("Cargo.toml").exists(), "rust");
    push_if(&mut langs, root.join("package.json").exists(), "typescript");
    push_if(&mut langs, root.join("go.mod").exists(), "go");
    push_if(
        &mut langs,
        root.join("pyproject.toml").exists() || root.join("setup.py").exists(),
        "python",
    );
    push_if(&mut langs, root.join("pom.xml").exists(), "java");
    langs
}

pub fn search_files(root: &Path, query: &str, limit: usize) -> anyhow::Result<Vec<FileMatch>> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let needle = query.to_lowercase();
    let mut matches = Vec::new();
    for entry in ignore::WalkBuilder::new(root).build() {
        let entry = entry?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        collect_file_matches(root, entry.path(), &needle, limit, &mut matches);
        if matches.len() >= limit {
            return Ok(matches);
        }
    }
    Ok(matches)
}

fn collect_file_matches(
    root: &Path,
    path: &Path,
    needle: &str,
    limit: usize,
    matches: &mut Vec<FileMatch>,
) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for (idx, line) in content.lines().enumerate() {
        if line.to_lowercase().contains(needle) {
            let rel = path.strip_prefix(root).unwrap_or(path);
            matches.push(FileMatch {
                path: rel.display().to_string(),
                line: idx as u32 + 1,
                text: line.trim().to_string(),
            });
            if matches.len() >= limit {
                break;
            }
        }
    }
}

fn file_rank(path: &str) -> u8 {
    if matches!(
        path,
        "Cargo.toml" | "package.json" | "go.mod" | "pyproject.toml"
    ) {
        0
    } else if path == "README.md" || path == "Makefile" {
        1
    } else if path.starts_with("src/") || path.starts_with("crates/") || path.starts_with("lib/") {
        2
    } else if path.starts_with("tests/") || path.ends_with("_test.rs") || path.ends_with(".test.ts")
    {
        3
    } else {
        5
    }
}

fn push_if(langs: &mut Vec<String>, condition: bool, lang: &str) {
    if condition {
        langs.push(lang.into());
    }
}
