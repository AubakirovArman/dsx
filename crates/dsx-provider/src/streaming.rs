//! Streaming types and SSE parser for DeepSeek V4 responses.

pub use crate::streaming_types::*;
use std::collections::BTreeMap;

// ── Accumulator for in-progress tool calls ──────────────────────────

#[derive(Debug, Default)]
struct ToolAccumulator {
    pending: BTreeMap<u32, ToolCallParts>,
}

#[derive(Debug, Default)]
struct ToolCallParts {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl ToolAccumulator {
    fn ingest(&mut self, deltas: &[StreamToolCallDelta]) -> Vec<ToolCallReady> {
        let mut ready = Vec::new();
        for delta in deltas {
            let parts = self.pending.entry(delta.index).or_default();
            if let Some(ref id) = delta.id {
                parts.id = Some(id.clone());
            }
            if let Some(ref func) = delta.function {
                if let Some(ref name) = func.name {
                    parts.name = Some(name.clone());
                }
                if let Some(ref args) = func.arguments {
                    parts.arguments.push_str(args);
                }
            }
            // A tool call is ready when we have id, name, and arguments are valid JSON
            if let Some(ref id) = parts.id
                && let Some(ref name) = parts.name
                && is_complete_json(&parts.arguments)
            {
                let tc = ToolCallReady {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: parts.arguments.clone(),
                };
                ready.push(tc);
                self.pending.remove(&delta.index);
            }
        }
        ready
    }
}

/// Heuristic: a JSON argument string is "complete" if it parses as valid JSON.
fn is_complete_json(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
}

// ── SSE line parser ─────────────────────────────────────────────────

/// Parse an SSE response body into a stream of `StreamEvent`s.
pub async fn parse_sse_stream(response: reqwest::Response) -> anyhow::Result<Vec<StreamEvent>> {
    let mut events = Vec::new();
    let mut acc = ToolAccumulator::default();
    let mut bytes = response.bytes_stream();

    use futures_util::StreamExt;
    let mut buffer = String::new();

    while let Some(chunk) = bytes.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Split on double newlines (SSE event boundary)
        while let Some(pos) = buffer.find("\n\n") {
            let event_str = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event_str.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with(':') || !line.starts_with("data: ") {
                    continue;
                }
                let data = &line["data: ".len()..];
                if data == "[DONE]" {
                    events.push(StreamEvent::Finish {
                        finish_reason: "stop".into(),
                        usage: None,
                    });
                    return Ok(events);
                }
                // Parse JSON chunk
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                    if chunk.choices.is_empty()
                        && let Some(usage) = chunk.usage.clone()
                    {
                        events.push(StreamEvent::Finish {
                            finish_reason: "usage".into(),
                            usage: Some(usage),
                        });
                    }
                    for choice in &chunk.choices {
                        // Reasoning content
                        if let Some(ref rc) = choice.delta.reasoning_content {
                            events.push(StreamEvent::Reasoning(rc.clone()));
                        }
                        // Content
                        if let Some(ref c) = choice.delta.content {
                            events.push(StreamEvent::Content(c.clone()));
                        }
                        // Tool calls: accumulate
                        if let Some(ref deltas) = choice.delta.tool_calls {
                            let ready = acc.ingest(deltas);
                            for tc in ready {
                                events.push(StreamEvent::ToolCall(tc));
                            }
                        }
                        // Finish
                        if let Some(ref fr) = choice.finish_reason {
                            events.push(StreamEvent::Finish {
                                finish_reason: fr.clone(),
                                usage: chunk.usage.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(events)
}

/// Parse SSE and call `on_event` for each parsed event in real-time.
pub async fn parse_sse_stream_callback<F>(
    response: reqwest::Response,
    mut on_event: F,
) -> anyhow::Result<()>
where
    F: FnMut(StreamEvent),
{
    let mut acc = ToolAccumulator::default();
    let mut bytes = response.bytes_stream();
    use futures_util::StreamExt;
    let mut buffer = String::new();

    while let Some(chunk) = bytes.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_str = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event_str.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with(':') || !line.starts_with("data: ") {
                    continue;
                }
                let data = &line["data: ".len()..];
                if data == "[DONE]" {
                    on_event(StreamEvent::Finish {
                        finish_reason: "stop".into(),
                        usage: None,
                    });
                    return Ok(());
                }
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                    if chunk.choices.is_empty()
                        && let Some(usage) = chunk.usage.clone()
                    {
                        on_event(StreamEvent::Finish {
                            finish_reason: "usage".into(),
                            usage: Some(usage),
                        });
                    }
                    for choice in &chunk.choices {
                        if let Some(ref rc) = choice.delta.reasoning_content {
                            on_event(StreamEvent::Reasoning(rc.clone()));
                        }
                        if let Some(ref c) = choice.delta.content {
                            on_event(StreamEvent::Content(c.clone()));
                        }
                        if let Some(ref deltas) = choice.delta.tool_calls {
                            let ready = acc.ingest(deltas);
                            for tc in ready {
                                on_event(StreamEvent::ToolCall(tc));
                            }
                        }
                        if let Some(ref fr) = choice.finish_reason {
                            on_event(StreamEvent::Finish {
                                finish_reason: fr.clone(),
                                usage: chunk.usage.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
