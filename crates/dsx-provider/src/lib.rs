//! DeepSeek V4 API client — OpenAI-compatible Chat Completions.

pub mod client;
pub mod streaming;
mod streaming_types;
pub mod types;

use dsx_core::types::ModelRoute;

/// Convert a ModelRoute into the API model name and thinking config.
pub fn model_config(route: ModelRoute) -> (&'static str, bool, Option<&'static str>) {
    match route {
        ModelRoute::ProMax => ("deepseek-v4-pro", true, Some("max")),
        ModelRoute::ProHigh => ("deepseek-v4-pro", true, Some("high")),
        ModelRoute::Flash => ("deepseek-v4-flash", false, None),
        ModelRoute::FlashThinking => ("deepseek-v4-flash", true, Some("high")),
    }
}
