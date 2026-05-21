//! DSX Config — layered config loader (global → project → team).

mod layers;
mod loader;
mod settings;

pub use loader::{load, load_for_project};
pub use settings::{
    AppConfig, AppSettings, CommandRule, ModelSettings, ModelSpec, PathSettings, PermissionsLayer,
    ProjectSettings, ProviderSettings, RoutingSettings, ScopeSettings,
};
