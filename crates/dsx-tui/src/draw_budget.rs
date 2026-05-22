//! Run budget display helpers.

use crate::App;

pub(crate) fn run_budget_badge(app: &App) -> String {
    let budget = &app.run_budget;
    if budget.max_tokens == 0 {
        return format!("{} {} tok", status_text(&budget.status), budget.used_tokens);
    }
    format!(
        "{} {} / {} tok, ${:.4}/${:.2}",
        status_text(&budget.status),
        budget.used_tokens,
        compact_tokens(budget.max_tokens),
        budget.estimated_cost_usd,
        budget.max_cost_usd
    )
}

pub(crate) fn run_budget_line(app: &App) -> String {
    format!("{}; {}", run_budget_badge(app), app.run_budget.last_update)
}

pub(crate) fn budget_status(
    used_tokens: u64,
    max_tokens: u64,
    cost: f64,
    max_cost: f64,
    running: bool,
) -> &'static str {
    let token_over = max_tokens > 0 && used_tokens > max_tokens;
    let cost_over = max_cost > 0.0 && cost > max_cost;
    if token_over || cost_over {
        return "over";
    }
    let token_near = max_tokens > 0 && used_tokens.saturating_mul(100) >= max_tokens * 90;
    let cost_near = max_cost > 0.0 && cost >= max_cost * 0.90;
    if token_near || cost_near {
        return "near";
    }
    if running { "ok" } else { "done" }
}

fn status_text(status: &str) -> &str {
    match status {
        "over" => "OVER",
        "near" => "NEAR",
        "ok" => "OK",
        "done" => "DONE",
        "running" => "RUN",
        "failed" => "FAIL",
        _ => "IDLE",
    }
}

fn compact_tokens(tokens: u64) -> String {
    if tokens >= 1_000 {
        format!("{}k", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_status_marks_near_and_over() {
        assert_eq!(budget_status(89, 100, 0.0, 2.0, true), "ok");
        assert_eq!(budget_status(90, 100, 0.0, 2.0, true), "near");
        assert_eq!(budget_status(101, 100, 0.0, 2.0, true), "over");
        assert_eq!(budget_status(1, 100, 1.9, 2.0, true), "near");
    }

    #[test]
    fn run_budget_badge_compacts_limits() {
        let mut app = App::new();
        app.run_budget.used_tokens = 42;
        app.run_budget.max_tokens = 750_000;
        app.run_budget.max_cost_usd = 2.0;
        app.run_budget.status = "ok".into();

        assert_eq!(run_budget_badge(&app), "OK 42 / 750k tok, $0.0000/$2.00");
    }
}
