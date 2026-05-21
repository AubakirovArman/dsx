//! DSX Core — shared types and traits for the DSX Code agent runtime.

pub mod error;
pub mod types;

/// Re-export the workspace result type
pub type Result<T> = anyhow::Result<T>;
