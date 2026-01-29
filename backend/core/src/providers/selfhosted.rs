use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;
use crate::types::ChatCompletionRequest;
use super::{ProviderAdapter, ChatStream};

pub struct SelfHostedAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl SelfHostedAdapter {
    pub fn new(client: Client, api_key: String, base_url: String) -> Self {
        Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
    
    /// Check if this is an Ollama instance by checking the base URL pattern
    fn is_ollama(&self) -> bool {
        self.base_url.contains(":11434") || self.base_url.contains("ollama")
    }
}

#[async_trait]
impl ProviderAdapter for SelfHostedAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // Detect if this is Ollama and use native API, otherwise use OpenAI-compatible
        if self.is_ollama() {
            self.stream_chat_ollama(req).await
        } else {
            self.stream_chat_openai_compat(req).await
        }
    }
}

impl SelfHostedAdapter {
    /// Ollama native API (/api/generate)
    /// Supports both Docker (host.docker.internal) and local (localhost) environments
    async fn stream_chat_ollama(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // Convert messages to a single prompt string for /api/generate
        let prompt = req.messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let payload = json!({
            "model": req.model,
            "prompt": prompt,
            "stream": true
        });

        // Try the configured URL first
        let url = format!("{}/api/generate", self.base_url);
        eprintln!("🦙 Ollama request to {} with model {}", url, req.model);

        let mut response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await;

        // If connection fails and we're using host.docker.internal, try localhost fallback
        // (supports running locally outside Docker)
        if response.is_err() && self.base_url.contains("host.docker.internal") {
            let fallback_url = self.base_url.replace("host.docker.internal", "localhost");
            let fallback_full = format!("{}/api/generate", fallback_url);
            eprintln!("⚠️  host.docker.internal failed, trying localhost fallback: {}", fallback_full);
            
            response = self.client
                .post(&fallback_full)
                .json(&payload)
                .send()
                .await;
        }
        // Vice versa: if using localhost and it fails, try host.docker.internal
        // (supports Docker when user configured localhost)
        else if response.is_err() && (self.base_url.contains("localhost") || self.base_url.contains("127.0.0.1")) {
            let fallback_url = self.base_url
                .replace("localhost", "host.docker.internal")
                .replace("127.0.0.1", "host.docker.internal");
            let fallback_full = format!("{}/api/generate", fallback_url);
            eprintln!("⚠️  localhost failed, trying host.docker.internal fallback: {}", fallback_full);
            
            response = self.client
                .post(&fallback_full)
                .json(&payload)
                .send()
                .await;
        }

        let response = response?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error {}: {}", status, body));
        }

        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();
                    
                    // Ollama /api/generate streams JSON objects, one per line
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                            // Ollama /api/generate format: {"response": "..."}
                            if let Some(resp) = value["response"].as_str() {
                                content.push_str(resp);
                            }
                        }
                    }
                    
                    Ok(content)
                })
        });

        Ok(Box::pin(parsed_stream))
    }

    /// OpenAI-compatible API (/v1/chat/completions) for other self-hosted solutions
    async fn stream_chat_openai_compat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut request_builder = self.client
            .post(&url)
            .json(&json!({
                "model": req.model,
                "messages": req.messages,
                "stream": true,
            }));

        // Add API key if provided (some self-hosted solutions don't require it)
        if !self.api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;
        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut content = String::new();
                    
                    // Parse SSE format (OpenAI-compatible)
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
