//! DSX Telemetry — local-only audit log and usage metering.
//!
//! No telemetry data leaves the machine. Local logs only.

use tracing_subscriber::EnvFilter;

/// Initialize telemetry with env-filtered tracing.
pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

/// Record a cost event (local log).
pub fn record_usage(model: &str, tokens: u64, cost: f64) {
    tracing::info!(
        model = model,
        tokens = tokens,
        cost_usd = cost,
        "API usage recorded"
    );
}
