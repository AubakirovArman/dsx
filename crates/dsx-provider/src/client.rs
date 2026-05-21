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
            .read_timeout(std::time::Duration::from_secs(45))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client: req_client,
            base_url: clean_base,
            api_key,
        }
    }

    /// Send a non-streaming chat request with exponential backoff auto-retry.
    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_err = anyhow::anyhow!("Unknown error");

        while attempts < max_attempts {
            attempts += 1;
            match self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(request)
                .timeout(std::time::Duration::from_secs(12))
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        if let Ok(text) = resp.text().await {
                            return Ok(text);
                        }
                    } else {
                        let status_code = status.as_u16();
                        let body = resp.text().await.unwrap_or_default();
                        last_err = anyhow::anyhow!("API error {}: {}", status_code, body);
                    }
                }
                Err(e) => {
                    last_err = e.into();
                }
            }

            if attempts < max_attempts {
                tokio::time::sleep(std::time::Duration::from_secs(attempts)).await;
            }
        }
        Err(last_err)
    }

    /// Stream a chat completion and parse SSE events with automatic retry.
    pub async fn chat_stream_events(
        &self,
        request: &ChatRequest,
    ) -> anyhow::Result<Vec<StreamEvent>> {
        let mut req = request.clone();
        req.stream = Some(true);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_err = anyhow::anyhow!("Unknown streaming error");

        while attempts < max_attempts {
            attempts += 1;
            let send_fut = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&req)
                .send();

            match tokio::time::timeout(std::time::Duration::from_secs(20), send_fut).await {
                Ok(Ok(resp)) => {
                    let status = resp.status();
                    if status.is_success() {
                        match parse_sse_stream(resp).await {
                            Ok(evs) => return Ok(evs),
                            Err(e) => {
                                last_err = e;
                            }
                        }
                    } else {
                        let status_code = status.as_u16();
                        let body = resp.text().await.unwrap_or_default();
                        last_err = anyhow::anyhow!("API error {}: {}", status_code, body);
                    }
                }
                Ok(Err(e)) => {
                    last_err = e.into();
                }
                Err(_) => {
                    last_err = anyhow::anyhow!("Connection timed out (no response headers received within 20s)");
                }
            }

            if attempts < max_attempts {
                tokio::time::sleep(std::time::Duration::from_secs(attempts)).await;
            }
        }
        Err(last_err)
    }

    /// Stream a chat completion with exponential backoff auto-retry and real-time user-facing notifications.
    pub async fn chat_stream_callback<F>(
        &self,
        request: &ChatRequest,
        mut on_event: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(StreamEvent),
    {
        let mut req = request.clone();
        req.stream = Some(true);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_err = anyhow::anyhow!("Unknown streaming callback error");

        while attempts < max_attempts {
            attempts += 1;
            let send_fut = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&req)
                .send();

            match tokio::time::timeout(std::time::Duration::from_secs(20), send_fut).await {
                Ok(Ok(resp)) => {
                    let status = resp.status();
                    if status.is_success() {
                        match parse_sse_stream_callback(resp, &mut on_event).await {
                            Ok(()) => return Ok(()),
                            Err(e) => {
                                last_err = e;
                            }
                        }
                    } else {
                        let status_code = status.as_u16();
                        let body = resp.text().await.unwrap_or_default();
                        last_err = anyhow::anyhow!("API error {}: {}", status_code, body);
                    }
                }
                Ok(Err(e)) => {
                    last_err = e.into();
                }
                Err(_) => {
                    last_err = anyhow::anyhow!("Connection timed out (no response headers received within 20s)");
                }
            }

            if attempts < max_attempts {
                let msg = match attempts {
                    1 => "⚠️ [Сервер перегружен / Очередь промпта превысила 20с. Переподключение...]\n",
                    2 => "⚠️ [Таймаут первого байта. Вторая попытка переподключения через 2 секунды...]\n",
                    _ => "⚠️ [Задержка сети. Финальная попытка подключения через 3 секунды...]\n",
                };
                on_event(StreamEvent::Reasoning(msg.to_string()));
                tokio::time::sleep(std::time::Duration::from_secs(attempts)).await;
            }
        }
        Err(last_err)
    }
}
