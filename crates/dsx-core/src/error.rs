use thiserror::Error;

#[derive(Error, Debug)]
pub enum DsxError {
    #[error("config error: {0}")]
    Config(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("permission denied: {0}")]
    Permission(String),
    #[error("patch error: {0}")]
    Patch(String),
    #[error("git error: {0}")]
    Git(String),
    #[error("session error: {0}")]
    Session(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
