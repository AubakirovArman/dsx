//! DSX Agent — concrete tool execution implementations.

pub mod command_scope;
pub mod command_tools;
pub mod file_tools;
pub mod mcp_tools;
pub mod patch_tools;

pub use command_tools::exec_run_command;
pub use file_tools::{exec_grep, exec_list_files, exec_read_file, exec_write_file};
pub use mcp_tools::{exec_mcp_call, exec_mcp_list_tools};
pub use patch_tools::exec_propose_patch;

pub(crate) fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        content.to_string()
    } else {
        let mut truncated = content[..max_chars].to_string();
        truncated.push_str(&format!(
            "\n\n... [truncated at {max_chars} chars, total {} chars]",
            content.len()
        ));
        truncated
    }
}
