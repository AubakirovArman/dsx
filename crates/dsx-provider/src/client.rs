//! DeepSeek V4 API HTTP client.

use reqwest::Client;
use crate::types::ChatRequest;
use crate::streaming::{StreamEvent, parse_sse_stream, parse_sse_stream_callback};

pub struct DeepSeekClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String) -> Self {
        Self::new_with_base(api_key, "https://api.deepseek.com".into())
    }

    pub fn new_with_base(api_key: String, base_url: String) -> Self {
        let mut clean_base = base_url.trim().to_string();
        if clean_base.is_empty() {
            clean_base = "https://api.deepseek.com".to_string();
        }
        let req_client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client: req_client,
            base_url: clean_base,
            api_key,
        }
    }

    /// Send a non-streaming chat request.
    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(request)
            .timeout(std::time::Duration::from_secs(12))
            .send()
            .await?;
        let text = resp.text().await?;
        Ok(text)
    }

    /// Stream a chat completion and parse SSE events.
    pub async fn chat_stream_events(
        &self,
        request: &ChatRequest,
    ) -> anyhow::Result<Vec<StreamEvent>> {
        let mut req = request.clone();
        req.stream = Some(true);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {body}", status.as_u16());
        }
        tracing::debug!(status = %status, "Streaming response received");
        parse_sse_stream(resp).await
    }

    /// Stream a chat completion and call `on_event` for each parsed event in real-time.
    pub async fn chat_stream_callback<F>(
        &self,
        request: &ChatRequest,
        on_event: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(StreamEvent),
    {
        let mut req = request.clone();
        req.stream = Some(true);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {body}", status.as_u16());
        }
        parse_sse_stream_callback(resp, on_event).await
    }
}
