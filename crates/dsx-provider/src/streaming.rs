//! Streaming types and SSE parser for DeepSeek V4 responses.

use serde::Deserialize;
use std::collections::BTreeMap;

// ── Raw deserialized SSE chunk ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub choices: Vec<StreamChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: StreamDelta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StreamDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<StreamToolCallDelta>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolCallDelta {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub reasoning_tokens: Option<u32>,
    #[serde(default)]
    pub prompt_cache_hit_tokens: Option<u32>,
    #[serde(default)]
    pub prompt_cache_miss_tokens: Option<u32>,
}

// ── High-level stream events for the agent loop ─────────────────────

/// A single event from the streaming API.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Reasoning tokens (thinking block).
    Reasoning(String),
    /// Content tokens (the actual answer).
    Content(String),
    /// A complete tool call is ready (accumulated from deltas).
    ToolCall(ToolCallReady),
    /// Stream ended with a finish reason and usage.
    Finish {
        finish_reason: String,
        usage: Option<Usage>,
    },
    /// An execution or API connection error.
    Error(String),
}

/// A complete tool call ready for execution.
#[derive(Debug, Clone)]
pub struct ToolCallReady {
    pub id: String,
    pub name: String,
    pub arguments: String, // raw JSON string
}

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
            if let Some(ref id) = parts.id {
                if let Some(ref name) = parts.name {
                    if is_complete_json(&parts.arguments) {
                        let tc = ToolCallReady {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: parts.arguments.clone(),
                        };
                        ready.push(tc);
                        self.pending.remove(&delta.index);
                    }
                }
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
pub async fn parse_sse_stream(
    response: reqwest::Response,
) -> anyhow::Result<Vec<StreamEvent>> {
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
                    continue;
                }
                // Parse JSON chunk
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
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
                    continue;
                }
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
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

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_complete_json() {
        assert!(!is_complete_json(""));
        assert!(!is_complete_json("{"));
        assert!(is_complete_json("{}"));
        assert!(is_complete_json(r#"{"path": "src/main.rs"}"#));
        assert!(is_complete_json(r#"{"pattern": "fn main"}"#));
    }

    #[test]
    fn test_tool_accumulator_basic() {
        let mut acc = ToolAccumulator::default();
        // Simulate streaming tool call deltas
        let deltas1 = vec![StreamToolCallDelta {
            index: 0,
            id: Some("call_1".into()),
            type_: None,
            function: Some(FunctionDelta {
                name: Some("read_file".into()),
                arguments: Some(r#"{"path":"#.into()),
            }),
        }];
        let r1 = acc.ingest(&deltas1);
        assert!(r1.is_empty(), "should not be ready yet");

        let deltas2 = vec![StreamToolCallDelta {
            index: 0,
            id: None,
            type_: None,
            function: Some(FunctionDelta {
                name: None,
                arguments: Some(r#""src/main.rs"}"#.into()),
            }),
        }];
        let r2 = acc.ingest(&deltas2);
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].name, "read_file");
        assert_eq!(r2[0].arguments, r#"{"path":"src/main.rs"}"#);
    }

    #[test]
    fn test_tool_accumulator_multiple_tools() {
        let mut acc = ToolAccumulator::default();
        // Two tools interleaved
        let deltas = vec![
            StreamToolCallDelta {
                index: 0,
                id: Some("call_a".into()),
                type_: None,
                function: Some(FunctionDelta {
                    name: Some("grep".into()),
                    arguments: Some(r#"{"pattern":"todo"}"#.into()),
                }),
            },
            StreamToolCallDelta {
                index: 1,
                id: Some("call_b".into()),
                type_: None,
                function: Some(FunctionDelta {
                    name: Some("read_file".into()),
                    arguments: Some(r#"{"path":"README.md"}"#.into()),
                }),
            },
        ];
        let ready = acc.ingest(&deltas);
        assert_eq!(ready.len(), 2, "both tools should be ready");
    }
}
