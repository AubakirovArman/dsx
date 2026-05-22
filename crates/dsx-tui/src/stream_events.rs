//! Stream event handling for TUI state.

use crate::{AgentStreamEvent, AgentTask, App, ChatMessage, ToolTimelineEntry};

impl App {
    /// Process a streaming event from the agent.
    pub fn handle_stream_event(&mut self, event: &AgentStreamEvent) {
        match event {
            AgentStreamEvent::Reasoning(r) => self.current_reasoning.push_str(r),
            AgentStreamEvent::ContentToken(token) => self.push_content_token(token),
            AgentStreamEvent::ToolResult {
                name,
                success,
                denied,
                risk,
                summary,
            } => self.handle_tool_result(name, *success, *denied, risk, summary),
            AgentStreamEvent::TranscriptCompact {
                removed_messages,
                retained_messages,
                estimated_tokens_saved,
            } => {
                self.handle_transcript_compact(
                    *removed_messages,
                    *retained_messages,
                    *estimated_tokens_saved,
                );
            }
            AgentStreamEvent::Usage {
                prompt_tokens,
                completion_tokens,
                reasoning_tokens,
                total_tokens,
            } => self.handle_usage(
                *prompt_tokens,
                *completion_tokens,
                *reasoning_tokens,
                *total_tokens,
            ),
            AgentStreamEvent::Done {
                answer: _ans,
                iterations,
                tokens,
                cost,
            } => self.handle_done(*iterations, *tokens, *cost),
            AgentStreamEvent::Error(err) => self.handle_error(err),
        }
    }

    fn push_content_token(&mut self, token: &str) {
        if !self.current_reasoning.is_empty() {
            self.current_reasoning.clear();
        }
        if let Some(last) = self.messages.last_mut()
            && last.role == "assistant"
        {
            last.content.push_str(token);
            return;
        }
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: token.into(),
        });
    }

    fn handle_tool_result(
        &mut self,
        name: &str,
        success: bool,
        denied: bool,
        risk: &str,
        summary: &str,
    ) {
        let short: String = summary.chars().take(150).collect();
        let status = if denied {
            "blocked"
        } else if success {
            "ok"
        } else {
            "failed"
        };
        if is_scope_violation(denied, risk, &short) {
            self.record_scope_violation(name, &short);
        }
        self.push_tool_event(name, status, &short);
        self.add_message("tool", &format!("{} {name} - {short}", icon_for(status)));
    }

    fn handle_transcript_compact(
        &mut self,
        removed_messages: usize,
        retained_messages: usize,
        estimated_tokens_saved: usize,
    ) {
        self.compaction_events += 1;
        self.compacted_messages += removed_messages as u64;
        self.estimated_tokens_saved += estimated_tokens_saved as u64;
        let summary = format!(
            "{removed_messages} msg compacted, ~{estimated_tokens_saved} tok saved, {retained_messages} retained"
        );
        self.push_tool_event("context_compact", "ok", &summary);
        self.add_message("system", &format!("Context compacted: {summary}"));
    }

    fn handle_usage(
        &mut self,
        prompt_tokens: u64,
        completion_tokens: u64,
        reasoning_tokens: u64,
        total_tokens: u64,
    ) {
        let counted = total_tokens.max(prompt_tokens + completion_tokens + reasoning_tokens);
        self.run_budget.used_tokens = self.run_budget.used_tokens.saturating_add(counted);
        self.run_budget.last_update = format!(
            "last request: {counted} tok (prompt {prompt_tokens}, completion {completion_tokens}, reasoning {reasoning_tokens})"
        );
        self.update_run_budget_status(true);
    }

    fn handle_done(&mut self, iterations: usize, tokens: u64, cost: f64) {
        self.tokens += tokens;
        self.cost += cost;
        self.run_budget.used_tokens = self.run_budget.used_tokens.max(tokens);
        self.run_budget.estimated_cost_usd = cost;
        self.run_budget.last_update = format!("completed: {tokens} reported tok, ${cost:.4}");
        self.update_run_budget_status(false);
        self.current_reasoning.clear();
        self.agent_task = AgentTask::Done(format!("{iterations} iterations, ${cost:.4}"));
        self.task_brief.done = format!("Completed in {iterations} iteration(s).");
        self.task_brief.last_changes = "Final assistant response recorded.".into();
        self.task_brief.next_step = "Review result or enter the next task.".into();
    }

    fn handle_error(&mut self, err: &str) {
        self.add_message("error", err);
        self.current_reasoning.clear();
        self.agent_task = AgentTask::Error(err.into());
        self.run_budget.status = if err.contains("budget") || err.contains("runaway") {
            "over".into()
        } else {
            "failed".into()
        };
        self.run_budget.last_update = err.chars().take(180).collect();
        self.task_brief.done = "Run failed.".into();
        self.task_brief.last_changes = err.chars().take(220).collect();
        self.task_brief.next_step = "Inspect the error and retry with a narrower task.".into();
    }

    fn record_scope_violation(&mut self, name: &str, summary: &str) {
        self.scope_violations += 1;
        self.last_scope_violation = format!("{name}: {summary}");
        self.scope_lock.warning = format!(
            "{} blocked scope escape(s). Stay inside {}.",
            self.scope_violations, self.scope_lock.active_scope
        );
    }

    fn push_tool_event(&mut self, name: &str, status: &str, summary: &str) {
        self.tool_timeline.push(ToolTimelineEntry {
            name: name.into(),
            status: status.into(),
            summary: summary.into(),
        });
        if self.tool_timeline.len() > 20 {
            let overflow = self.tool_timeline.len() - 20;
            self.tool_timeline.drain(0..overflow);
        }
        self.task_brief.done = format!("Tool {name} finished with status {status}.");
        self.task_brief.last_changes = summary.into();
        self.task_brief.next_step = next_step_for(status);
        let active = self.task_brief.active_scope.clone();
        let next_step = self.task_brief.next_step.clone();
        self.upsert_folder_note(&active, summary, &next_step);
    }

    fn update_run_budget_status(&mut self, running: bool) {
        self.run_budget.status = crate::draw_budget::budget_status(
            self.run_budget.used_tokens,
            self.run_budget.max_tokens,
            self.run_budget.estimated_cost_usd,
            self.run_budget.max_cost_usd,
            running,
        )
        .into();
    }
}

fn is_scope_violation(denied: bool, _risk: &str, summary: &str) -> bool {
    denied
        && (summary.contains("active scope")
            || summary.contains("path traversal blocked")
            || summary.contains("leaves active scope"))
}

fn icon_for(status: &str) -> &'static str {
    match status {
        "ok" => "OK",
        "blocked" => "BLOCKED",
        _ => "FAIL",
    }
}

fn next_step_for(status: &str) -> String {
    match status {
        "ok" => "Continue from the latest tool result.".into(),
        "blocked" => "Retry with a path inside the active scope.".into(),
        _ => "Review failed tool output before continuing.".into(),
    }
}
