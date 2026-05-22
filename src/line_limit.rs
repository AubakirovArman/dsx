//! Rust source line-limit scanning shared by doctor and workspace health checks.

use std::path::{Path, PathBuf};

pub const MAX_RS_LINES: usize = 300;
pub const PRESSURE_RS_LINES: usize = 270;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLineCount {
    pub path: PathBuf,
    pub lines: usize,
}

pub fn rust_line_violations(root: &Path, max_lines: usize) -> anyhow::Result<Vec<FileLineCount>> {
    let mut counts = rust_line_counts(root)?;
    counts.retain(|item| item.lines > max_lines);
    sort_line_counts(&mut counts);
    Ok(counts)
}

pub fn rust_line_pressure(
    root: &Path,
    warn_at: usize,
    max_lines: usize,
) -> anyhow::Result<Vec<FileLineCount>> {
    let mut counts = rust_line_counts(root)?;
    counts.retain(|item| item.lines >= warn_at && item.lines <= max_lines);
    sort_line_counts(&mut counts);
    Ok(counts)
}

fn rust_line_counts(root: &Path) -> anyhow::Result<Vec<FileLineCount>> {
    let mut out = Vec::new();
    visit_rust_files(root, root, &mut out)?;
    Ok(out)
}

fn visit_rust_files(root: &Path, dir: &Path, out: &mut Vec<FileLineCount>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if !is_skipped_dir(&path) {
                visit_rust_files(root, &path, out)?;
            }
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(FileLineCount {
                path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                lines: std::fs::read_to_string(&path)?.lines().count(),
            });
        }
    }
    Ok(())
}

fn sort_line_counts(counts: &mut [FileLineCount]) {
    counts.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.path.cmp(&b.path)));
}

fn is_skipped_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    matches!(name, ".git" | ".dsx" | "target")
}
