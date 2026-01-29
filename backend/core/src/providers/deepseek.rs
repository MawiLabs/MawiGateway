use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;
use crate::types::ChatCompletionRequest;
use super::{ProviderAdapter, ChatStream};
use std::sync::{Arc, Mutex};

pub struct DeepSeekAdapter {
    client: Client,
    api_key: String,
}

impl DeepSeekAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
        }
    }
}

#[async_trait]
impl ProviderAdapter for DeepSeekAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // DeepSeek uses OpenAI-compatible API
        let url = "https://api.deepseek.com/v1/chat/completions";

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

        // Check response status before streaming
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "DeepSeek API error ({}): {}",
                status.as_u16(),
                error_body
            ));
        }

        let stream = response.bytes_stream();
        
        // Buffer for handling partial SSE lines across chunks
        let buffer = Arc::new(Mutex::new(String::new()));
        let buffer_clone = buffer.clone();
        
        let parsed_stream = stream.map(move |chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();
                    
                    // Append new data to buffer
                    let mut buf = buffer_clone.lock().unwrap();
                    buf.push_str(&text);
                    
                    // Process complete lines from buffer
                    let mut remaining = String::new();
                    for line in buf.lines() {
                        let line = line.trim();
                        
                        // Skip empty lines and done signal
                        if line.is_empty() {
                            continue;
                        }
                        if line == "data: [DONE]" {
                            continue;
                        }
                        
                        // Parse SSE data lines
                        if let Some(data) = line.strip_prefix("data: ") {
                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(value) => {
                                    if let Some(delta_content) = value["choices"][0]["delta"]["content"].as_str() {
                                        content.push_str(delta_content);
                                    }
                                    // Check for error in response
                                    if let Some(error) = value.get("error") {
                                        eprintln!("DeepSeek API error in stream: {:?}", error);
                                    }
                                }
                                Err(_) => {
                                    // Incomplete JSON - save for next chunk
                                    remaining.push_str(line);
                                    remaining.push('\n');
                                }
                            }
                        }
                    }
                    
                    // Keep incomplete data for next iteration
                    *buf = remaining;
                    
                    Ok(content)
                })
        });

        Ok(Box::pin(parsed_stream))
    }
}
