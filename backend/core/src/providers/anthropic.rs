use super::{ChatStream, ProviderAdapter};
use crate::error::{classify_reqwest_error, classify_response};
use crate::types::ChatCompletionRequest;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;

const PROVIDER: &str = "anthropic";

pub struct AnthropicAdapter {
    client: Client,
    api_key: String,
    base_url: String,
    version: String,
}

impl AnthropicAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            version: "2023-06-01".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        let system_message = req
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let messages: Vec<_> = req
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "stream": true,
            "max_tokens": req.max_tokens.unwrap_or(1024),
            "temperature": req.temperature,
        });

        if let Some(sys) = system_message {
            body.as_object_mut()
                .unwrap()
                .insert("system".to_string(), json!(sys));
        }

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::Error::new(classify_reqwest_error(PROVIDER, e)))?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let stream = response.bytes_stream();

        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .map(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();

                    for line in text.lines() {
                        if !line.starts_with("data: ") {
                            continue;
                        }

                        let json_str = line.strip_prefix("data: ").unwrap_or("").trim();
                        if json_str.is_empty() || json_str == "[DONE]" {
                            continue;
                        }

                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                            // Anthropic SSE structure:
                            // type: "content_block_delta" -> delta: { type: "text_delta", text: "..." }
                            if value["type"] == "content_block_delta" {
                                if let Some(text_content) = value["delta"]["text"].as_str() {
                                    content.push_str(text_content);
                                }
                            }
                        }
                    }
                    content
                })
        });

        Ok(Box::pin(parsed_stream))
    }
}
