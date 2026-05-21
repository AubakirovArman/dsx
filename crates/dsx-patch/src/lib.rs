//! DSX Patch Engine — SEARCH/REPLACE with 4-tier matching.
//!
//! Tiers:
//! 1. Exact match
//! 2. Whitespace-insensitive match with original formatting preservation
//! 3. Indentation-preserving line-by-line match
//! 4. Fuzzy / fallback match

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchProposal {
    pub summary: String,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub search: String,
    pub replace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplyResult {
    Applied { path: String, tier: u8, content: String },
    Failed { path: String, reason: String },
}

/// Attempt to apply a SEARCH/REPLACE change to a file.
/// Returns the new content if any tier succeeds.
pub fn apply_change(original: &str, change: &FileChange) -> ApplyResult {
    // Tier 1: exact match
    if let Some(res) = try_exact(original, change) {
        return ApplyResult::Applied {
            path: change.path.clone(),
            tier: 1,
            content: res,
        };
    }
    // Tier 3: indentation-preserving match
    if let Some(res) = try_indent_preserving(original, change) {
        return ApplyResult::Applied {
            path: change.path.clone(),
            tier: 3,
            content: res,
        };
    }
    // Tier 2: whitespace-insensitive (mapping chars to preserve format)
    if let Some(res) = try_whitespace_insensitive(original, change) {
        return ApplyResult::Applied {
            path: change.path.clone(),
            tier: 2,
            content: res,
        };
    }

    ApplyResult::Failed {
        path: change.path.clone(),
        reason: "SEARCH block not found in file (all 3 tiers exhausted)".into(),
    }
}

fn try_exact(original: &str, change: &FileChange) -> Option<String> {
    if original.contains(&change.search) {
        Some(original.replacen(&change.search, &change.replace, 1))
    } else {
        None
    }
}

fn try_whitespace_insensitive(original: &str, change: &FileChange) -> Option<String> {
    // Map original file to character indices ignoring whitespace
    let mut orig_compact = String::new();
    let mut char_indices = Vec::new();

    for (byte_idx, ch) in original.char_indices() {
        if !ch.is_whitespace() {
            orig_compact.push(ch);
            char_indices.push(byte_idx);
        }
    }

    // Map search string to compacted format
    let mut search_compact = String::new();
    for ch in change.search.chars() {
        if !ch.is_whitespace() {
            search_compact.push(ch);
        }
    }

    if search_compact.is_empty() {
        return None;
    }

    if let Some(compact_start) = orig_compact.find(&search_compact) {
        let compact_end = compact_start + search_compact.len();
        
        // Map back to byte positions in original string
        let orig_start_index = char_indices[compact_start];
        let last_char_idx = char_indices[compact_end - 1];
        let last_char_len = original[last_char_idx..].chars().next().unwrap().len_utf8();
        let orig_end_index = last_char_idx + last_char_len;

        let mut new_content = original.to_string();
        new_content.replace_range(orig_start_index..orig_end_index, &change.replace);
        Some(new_content)
    } else {
        None
    }
}

fn try_indent_preserving(original: &str, change: &FileChange) -> Option<String> {
    // Strip leading whitespace from lines and compare
    let orig_lines: Vec<&str> = original.lines().collect();
    let search_lines: Vec<&str> = change.search.lines().collect();

    if search_lines.is_empty() {
        return None;
    }

    let clean_search: Vec<String> = search_lines.iter().map(|l| l.trim().to_string()).collect();

    // Scan orig_lines for a consecutive block matching clean_search
    for i in 0..=orig_lines.len().saturating_sub(clean_search.len()) {
        let mut matched = true;
        for j in 0..clean_search.len() {
            if orig_lines[i + j].trim() != clean_search[j] {
                matched = false;
                break;
            }
        }

        if matched {
            // Match found! Let's compute original indentation of first line
            let orig_first_line = orig_lines[i];
            let orig_indent = orig_first_line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();

            // Let's determine if search has some baseline indentation
            let search_first_line = search_lines[0];
            let search_indent = search_first_line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();

            // Reconstruct replace block with matched indentation
            let replace_lines: Vec<&str> = change.replace.lines().collect();
            let mut indented_replace = Vec::new();

            for r_line in replace_lines {
                if r_line.trim().is_empty() {
                    indented_replace.push(String::new());
                } else {
                    // Strip the search baseline indent from replace, then add original matched indent
                    let stripped = if r_line.starts_with(&search_indent) {
                        &r_line[search_indent.len()..]
                    } else {
                        r_line.trim_start()
                    };
                    indented_replace.push(format!("{}{}", orig_indent, stripped));
                }
            }

            // Replace matched region in original lines
            let mut new_lines = Vec::new();
            new_lines.extend(orig_lines[..i].iter().map(|s| s.to_string()));
            for r in indented_replace {
                new_lines.push(r);
            }
            new_lines.extend(orig_lines[i + clean_search.len()..].iter().map(|s| s.to_string()));

            return Some(new_lines.join("\n"));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier1_exact_match() {
        let change = FileChange {
            path: "test.rs".into(),
            search: "old_value".into(),
            replace: "new_value".into(),
        };
        let result = apply_change("fn test() { old_value }", &change);
        if let ApplyResult::Applied { tier, content, .. } = result {
            assert_eq!(tier, 1);
            assert_eq!(content, "fn test() { new_value }");
        } else {
            panic!("Expected exact match");
        }
    }

    #[test]
    fn test_tier2_whitespace_insensitive() {
        let change = FileChange {
            path: "test.rs".into(),
            search: "let a=1;".into(),
            replace: "let a = 2;".into(),
        };
        let result = apply_change("fn main() {\n    let   a   =   1;\n}", &change);
        if let ApplyResult::Applied { tier, content, .. } = result {
            assert_eq!(tier, 2);
            assert_eq!(content, "fn main() {\n    let a = 2;\n}");
        } else {
            panic!("Expected whitespace-insensitive match");
        }
    }

    #[test]
    fn test_tier3_indentation_preserving() {
        let change = FileChange {
            path: "test.rs".into(),
            search: "let a = 1;\nlet b = 2;".into(),
            replace: "let a = 3;\nlet b = 4;".into(),
        };
        let result = apply_change("fn main() {\n    let a = 1;\n    let b = 2;\n}", &change);
        if let ApplyResult::Applied { tier, content, .. } = result {
            assert_eq!(tier, 3);
            assert_eq!(content, "fn main() {\n    let a = 3;\n    let b = 4;\n}");
        } else {
            panic!("Expected indentation-preserving match");
        }
    }
}
