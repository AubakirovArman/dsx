//! Tests for provider-to-TUI stream event conversion.

#[cfg(test)]
mod tests {
    #[test]
    fn finish_usage_maps_to_tui_usage_event() {
        let event = dsx_provider::streaming::StreamEvent::Finish {
            finish_reason: "stop".into(),
            usage: Some(dsx_provider::streaming::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                reasoning_tokens: Some(3),
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
            }),
        };

        let converted = crate::event_convert::convert_event(&event);

        match converted {
            dsx_tui::AgentStreamEvent::Usage {
                prompt_tokens,
                completion_tokens,
                reasoning_tokens,
                total_tokens,
            } => {
                assert_eq!(prompt_tokens, 10);
                assert_eq!(completion_tokens, 5);
                assert_eq!(reasoning_tokens, 3);
                assert_eq!(total_tokens, 15);
            }
            _ => panic!("expected usage event"),
        }
    }
}
