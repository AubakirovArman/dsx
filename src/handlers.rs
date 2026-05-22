//! DSX CLI — execution handlers for subcommands.

#[path = "handlers_agent.rs"]
mod agent;
#[path = "handlers_index.rs"]
mod index;
#[path = "handlers_mcp.rs"]
mod mcp;
#[path = "handlers_scope.rs"]
mod scope;

pub use agent::{run_edit, run_eval, run_plan};
pub use index::{run_index_build, run_index_search};
pub use mcp::{run_mcp_call, run_mcp_list};
pub use scope::{list_sessions, run_scope_preview, task_preview};
