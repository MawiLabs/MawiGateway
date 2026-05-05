use super::{ChatStream, ProviderAdapter};
use crate::types::ChatCompletionRequest;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;

pub struct SelfHostedAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl SelfHostedAdapter {
    pub fn new(client: Client, api_key: String, base_url: String) -> Self {
        // Normalize trailing slash + a trailing `/v1` if the user pasted the
        // full versioned URL (OpenRouter, vLLM, llama.cpp, etc. all advertise
        // their base as `…/api/v1`). The adapter then appends its own
        // `/v1/chat/completions` so we'd otherwise produce `…/v1/v1/...`.
        let trimmed = base_url.trim_end_matches('/');
        let normalized = trimmed.strip_suffix("/v1").unwrap_or(trimmed).to_string();
        Self {
            client,
            api_key,
            base_url: normalized,
        }
    }

    /// Check if this is an Ollama instance by checking the base URL pattern.
    /// Local-port + the literal "ollama" substring covers default installs
    /// and most reverse-proxied ones; explicit OpenAI-compat services
    /// (vLLM, OpenRouter, Together, Anyscale) won't match either heuristic.
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
    async fn stream_chat_ollama(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatStream, anyhow::Error> {
        // Convert messages to a single prompt string for /api/generate
        let prompt = req
            .messages
            .iter()
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

        let mut response = self.client.post(&url).json(&payload).send().await;

        // If connection fails and we're using host.docker.internal, try localhost fallback
        // (supports running locally outside Docker)
        if response.is_err() && self.base_url.contains("host.docker.internal") {
            let fallback_url = self.base_url.replace("host.docker.internal", "localhost");
            let fallback_full = format!("{}/api/generate", fallback_url);
            eprintln!(
                "⚠️  host.docker.internal failed, trying localhost fallback: {}",
                fallback_full
            );

            response = self.client.post(&fallback_full).json(&payload).send().await;
        }
        // Vice versa: if using localhost and it fails, try host.docker.internal
        // (supports Docker when user configured localhost)
        else if response.is_err()
            && (self.base_url.contains("localhost") || self.base_url.contains("127.0.0.1"))
        {
            let fallback_url = self
                .base_url
                .replace("localhost", "host.docker.internal")
                .replace("127.0.0.1", "host.docker.internal");
            let fallback_full = format!("{}/api/generate", fallback_url);
            eprintln!(
                "⚠️  localhost failed, trying host.docker.internal fallback: {}",
                fallback_full
            );

            response = self.client.post(&fallback_full).json(&payload).send().await;
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
                .map(|bytes| {
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

                    content
                })
        });

        Ok(Box::pin(parsed_stream))
    }

    /// OpenAI-compatible API (/v1/chat/completions) for other self-hosted solutions
    async fn stream_chat_openai_compat(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatStream, anyhow::Error> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut request_builder = self.client.post(&url).json(&json!({
            "model": req.model,
            "messages": req.messages,
            "stream": true,
        }));

        // Add API key if provided (some self-hosted solutions don't require it)
        if !self.api_key.is_empty() {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;
        let stream = response.bytes_stream();

        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .map(|bytes| {
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
                                if let Some(delta_content) =
                                    value["choices"][0]["delta"]["content"].as_str()
                                {
                                    content.push_str(delta_content);
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

#[cfg(test)]
mod tests {
    //! Lock down the base-URL normalization. Real-world users paste the
    //! base URL from an upstream's docs, and the upstreams don't agree:
    //! Ollama advertises `:11434`, vLLM advertises `…:8000`, OpenRouter
    //! advertises `…/api/v1`, llama.cpp advertises `…:8080/v1`. The
    //! adapter has to land on one canonical shape so its own
    //! `/v1/chat/completions` append doesn't double up.
    use super::*;

    fn adapter(base: &str) -> SelfHostedAdapter {
        SelfHostedAdapter::new(Client::new(), "k".into(), base.to_string())
    }

    #[test]
    fn strips_trailing_slash() {
        assert_eq!(
            adapter("http://localhost:11434/").base_url,
            "http://localhost:11434"
        );
    }

    #[test]
    fn strips_versioned_suffix_for_openrouter_style() {
        // The big one — users paste the full `…/api/v1` URL from
        // OpenRouter's quickstart and it would otherwise produce
        // `…/api/v1/v1/chat/completions` (404).
        assert_eq!(
            adapter("https://openrouter.ai/api/v1").base_url,
            "https://openrouter.ai/api"
        );
        assert_eq!(
            adapter("https://openrouter.ai/api/v1/").base_url,
            "https://openrouter.ai/api"
        );
    }

    #[test]
    fn leaves_non_versioned_urls_alone() {
        // Ollama and bare OpenAI-compat hosts shouldn't get touched.
        assert_eq!(
            adapter("http://localhost:11434").base_url,
            "http://localhost:11434"
        );
        assert_eq!(
            adapter("https://api.together.xyz").base_url,
            "https://api.together.xyz"
        );
    }

    #[test]
    fn ollama_detection() {
        assert!(adapter("http://localhost:11434").is_ollama());
        assert!(adapter("http://my-ollama-host:11434").is_ollama());
        assert!(adapter("http://ollama.internal").is_ollama());
        // OpenAI-compat services should NOT match.
        assert!(!adapter("https://openrouter.ai/api").is_ollama());
        assert!(!adapter("https://api.together.xyz").is_ollama());
    }
}
