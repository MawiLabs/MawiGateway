use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;
use crate::types::ChatCompletionRequest;
use super::{ProviderAdapter, ChatStream};

pub struct MistralAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl MistralAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.mistral.ai/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for MistralAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": req.model,
                "messages": req.messages,
                "stream": true,
                "temperature": req.temperature,
                "max_tokens": req.max_tokens,
                "safe_prompt": false // Mistral specific param
            }))
            .send()
            .await?;

        if !response.status().is_success() {
             let status = response.status();
             let error_text = response.text().await.unwrap_or_default();
             return Err(anyhow::anyhow!("Mistral API error: {} - {}", status, error_text));
        }

        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
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
                            if let Some(text_content) = value["choices"][0]["delta"]["content"].as_str() {
                                content.push_str(text_content);
                            }
                        }
                    }
                    Ok(content)
                })
        });

        Ok(Box::pin(parsed_stream))
    }
}
