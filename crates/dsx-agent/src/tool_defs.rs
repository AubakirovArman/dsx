//! Tool schema conversion and compact tool-result summaries.

use crate::types::ToolResult;
use dsx_provider::types::{FunctionDef, ToolDef};

pub fn summarize_tool_result(result: &ToolResult) -> String {
    let mut summary: String = result.content.chars().take(300).collect();
    if result.content.chars().count() > 300 {
        summary.push_str("...");
    }
    summary
}

pub fn summarize_tool_results(results: &[ToolResult]) -> String {
    if results.is_empty() {
        return "none".into();
    }

    results
        .iter()
        .rev()
        .take(3)
        .map(|result| {
            let status = if result.success { "ok" } else { "failed" };
            let summary = summarize_tool_result(result);
            format!("{}={}: {}", result.name, status, summary)
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn build_tool_defs() -> Vec<ToolDef> {
    dsx_tools::ToolRegistry::builtin_specs()
        .into_iter()
        .map(|spec| ToolDef {
            type_: "function".into(),
            function: FunctionDef {
                name: spec.name.clone(),
                description: spec.description.clone(),
                parameters: spec.parameters.clone(),
            },
        })
        .collect()
}
