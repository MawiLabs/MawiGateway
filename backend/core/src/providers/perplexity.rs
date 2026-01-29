use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;
use crate::types::ChatCompletionRequest;
use super::{ProviderAdapter, ChatStream};

pub struct PerplexityAdapter {
    client: Client,
    api_key: String,
}

impl PerplexityAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
        }
    }
}

#[async_trait]
impl ProviderAdapter for PerplexityAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // Perplexity uses OpenAI-compatible API
        let url = "https://api.perplexity.ai/chat/completions";

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": req.model,
                "messages": req.messages,
                "stream": true,
            }))
            .send()
            .await?;

        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();
                    
                    // Parse SSE format (same as OpenAI)
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() || line == "data: [DONE]" {
                            continue;
                        }
                        
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(delta_content) = value["choices"][0]["delta"]["content"].as_str() {
                                    content.push_str(delta_content);
                                }
                            }
                        }
                    }
                    
                    Ok(content)
                })
        });

        Ok(Box::pin(parsed_stream))
    }
}
