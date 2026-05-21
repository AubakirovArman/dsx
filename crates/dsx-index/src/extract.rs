//! Lightweight structural symbol extraction.

use crate::types::Symbol;

pub fn extract_symbols(content: &str, file_ext: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    if !matches!(file_ext, "rs" | "ts" | "js" | "py") {
        return symbols;
    }

    for (idx, line) in lines.iter().map(|line| line.trim()).enumerate() {
        match file_ext {
            "rs" => extract_rust(line, idx, &lines, &mut symbols),
            "py" => extract_python(line, idx, &lines, &mut symbols),
            "ts" | "js" => extract_js(line, idx, &lines, &mut symbols),
            _ => {}
        }
    }
    symbols
}

fn extract_rust(line: &str, idx: usize, lines: &[&str], symbols: &mut Vec<Symbol>) {
    if line.contains("fn ")
        && (line.starts_with("fn ")
            || line.starts_with("pub ")
            || line.starts_with("async ")
            || line.starts_with("pub async "))
    {
        push_braced_symbol(line, idx, lines, "fn ", "function", symbols);
    } else if line.contains("struct ") && (line.starts_with("struct ") || line.starts_with("pub "))
    {
        push_braced_symbol(line, idx, lines, "struct ", "struct", symbols);
    }
}

fn extract_python(line: &str, idx: usize, lines: &[&str], symbols: &mut Vec<Symbol>) {
    if let Some(stripped) = line.strip_prefix("def ") {
        let name = stripped.split('(').next().unwrap_or("").trim();
        push_python_symbol(line, idx, lines, name, "function", symbols);
    } else if let Some(stripped) = line.strip_prefix("class ") {
        let name = stripped
            .split(':')
            .next()
            .unwrap_or("")
            .split('(')
            .next()
            .unwrap_or("")
            .trim();
        push_python_symbol(line, idx, lines, name, "class", symbols);
    }
}

fn extract_js(line: &str, idx: usize, lines: &[&str], symbols: &mut Vec<Symbol>) {
    if line.contains("class ") && (line.starts_with("class ") || line.starts_with("export ")) {
        push_braced_symbol(line, idx, lines, "class ", "class", symbols);
    } else if line.contains("function ")
        && (line.starts_with("function ")
            || line.starts_with("export ")
            || line.starts_with("async "))
    {
        push_braced_symbol(line, idx, lines, "function ", "function", symbols);
    }
}

fn push_braced_symbol(
    line: &str,
    idx: usize,
    lines: &[&str],
    keyword: &str,
    kind: &str,
    symbols: &mut Vec<Symbol>,
) {
    if let Some(name) = extract_name(line, keyword) {
        symbols.push(Symbol {
            name,
            kind: kind.into(),
            start_line: idx as u32 + 1,
            end_line: find_closing_brace(lines, idx) as u32 + 1,
            signature: line.to_string(),
        });
    }
}

fn push_python_symbol(
    line: &str,
    idx: usize,
    lines: &[&str],
    name: &str,
    kind: &str,
    symbols: &mut Vec<Symbol>,
) {
    if !name.is_empty() {
        symbols.push(Symbol {
            name: name.into(),
            kind: kind.into(),
            start_line: idx as u32 + 1,
            end_line: find_python_block_end(lines, idx) as u32 + 1,
            signature: line.trim_end_matches(':').to_string(),
        });
    }
}

fn extract_name(line: &str, keyword: &str) -> Option<String> {
    let idx = line.find(keyword)?;
    let name = line[idx + keyword.len()..]
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    (!name.is_empty()).then(|| name.to_string())
}

fn find_closing_brace(lines: &[&str], start_idx: usize) -> usize {
    let mut brace_count = 0;
    let mut found_first = false;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
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
    let first_indent = lines[start_idx]
        .chars()
        .take_while(|c| c.is_whitespace())
        .count();

    for (idx, line) in lines.iter().enumerate().skip(start_idx + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if indent <= first_indent && !line.trim().starts_with('#') {
            return idx - 1;
        }
    }
    lines.len() - 1
}
