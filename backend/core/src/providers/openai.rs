use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;
use crate::types::ChatCompletionRequest;
use super::{ProviderAdapter, ChatStream};

pub struct OpenAIAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for OpenAIAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // Auto-route multimodal models to /responses endpoint
        if req.modality.as_deref() == Some("multimodal") {
            #[cfg(debug_assertions)]
            eprintln!("🌐 Auto-routing multimodal model to /responses endpoint");
            return self.stream_responses(req).await;
        }

        // Only include reasoning_effort for o1/o3 models that support it
        let is_reasoning_model = req.model.starts_with("o1") || req.model.starts_with("o3") || req.model.contains("gpt-5");
        
        let mut body = json!({
            "model": req.model,
            "messages": req.messages,
            "stream": true,
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
            "response_format": req.response_format,
        });
        
        // Only add reasoning_effort for models that support it
        if is_reasoning_model {
            if let Some(ref effort) = req.reasoning_effort {
                body["reasoning_effort"] = json!(effort);
            }
        }
        
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    // Sanitized logging: Only log that we received a chunk
                    #[cfg(debug_assertions)]
                    eprintln!("Received chunk: {} bytes", text.len());

                    let mut content = String::new();
                    
                    // Parse Server-Sent Events (SSE) format
                    for line in text.lines() {
                        if !line.starts_with("data: ") {
                            continue;
                        }
                        
                        let json_str = line.strip_prefix("data: ").unwrap_or("");
                        
                        // Check for stream end
                        if json_str.trim() == "[DONE]" {
                            continue;
                        }
                        
                        // Parse JSON and extract content
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(text_content) = value["choices"][0]["delta"]["content"].as_str() {
                                content.push_str(text_content);
                            }
                        }
                    }
                    Ok(content)
                })
        }); // REMOVED .filter() to see all debug output

        Ok(Box::pin(parsed_stream))
    }

    async fn generate_video(&self, req: &crate::types::VideoGenerationRequest) -> Result<crate::types::VideoGenerationResponse, anyhow::Error> {
        #[cfg(debug_assertions)]
        eprintln!("🎬 OpenAI Sora video generation - model: {}", req.model);
        
        // Parse size and duration (OpenAI supports 4, 8, or 12 seconds)
        let size = req.size.clone().unwrap_or_else(|| "1280x720".to_string());
        let duration = req.duration.unwrap_or(8).to_string(); // Default to 8 seconds
        
        // OpenAI Sora uses multipart/form-data
        let form = reqwest::multipart::Form::new()
            .text("prompt", req.prompt.clone())
            .text("model", req.model.clone())
            .text("size", size)
            .text("seconds", duration);
        
        let response = self.client
            .post(format!("{}/videos", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI Sora API error {}: {}", status, error_text));
        }

        let json: serde_json::Value = response.json().await?;
        eprintln!("📥 OpenAI Sora response: [JSON hidden]");
        
        // Return video ID immediately
        let video_id = json["id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No video ID in response"))?;
        
        eprintln!("🔄 Video created: {} - returning immediately", video_id);
        
        Ok(crate::types::VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", video_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, video_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        // Poll OpenAI video status
        let poll_url = format!("{}/videos/{}", self.base_url, video_id);
        
        let response = self.client
            .get(&poll_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to poll video: {}", error_text);
        }
        
        let video_status: serde_json::Value = response.json().await?;
        let status = video_status["status"].as_str().unwrap_or("unknown");
        
        // If completed, return video URL
        if status == "completed" {
            let video_url = format!("{}/videos/{}", self.base_url, video_id);
            
            return Ok(serde_json::json!({
                "status": "succeeded",
                "video_url": video_url
            }));
        }
        
        Ok(serde_json::json!({
            "status": status
        }))
    }

    async fn get_video_content(&self, video_id: &str) -> Result<Vec<u8>, anyhow::Error> {
        // OpenAI returns the video directly from the video ID endpoint
        let video_url = format!("{}/videos/{}/content", self.base_url, video_id);
        
        let response = self.client
            .get(&video_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to fetch video content: {}", error_text);
        }
        
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

// Additional methods for OpenAIAdapter (not part of ProviderAdapter trait)
impl OpenAIAdapter {
    /// Stream from OpenAI /responses endpoint (GPT-5 multimodal)
    pub async fn stream_responses(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        #[cfg(debug_assertions)]
        eprintln!("🌐 Using OpenAI /responses endpoint for multimodal model: {}", req.model);
        
        let body = json!({
            "model": req.model,
            "messages": req.messages,
            "stream": true,
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });
        
        let response = self.client
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI /responses error: {}", error_text));
        }

        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    // Sanitized logging
                    #[cfg(debug_assertions)]
                    eprintln!("Received responses chunk: {} bytes", text.len());
                    
                    let mut content = String::new();
                    
                    // Parse Server-Sent Events (SSE) format
                    for line in text.lines() {
                        if !line.starts_with("data: ") {
                            continue;
                        }
                        
                        let json_str = line.strip_prefix("data: ").unwrap_or("");
                        
                        // Check for stream end
                        if json_str.trim() == "[DONE]" {
                            continue;
                        }
                        
                        // Parse JSON and extract content
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                            // Handle different event types
                            if let Some(event_type) = value["type"].as_str() {
                                match event_type {
                                    "response.output_text.delta" => {
                                        // Text chunk
                                        if let Some(text_delta) = value["delta"].as_str() {
                                            content.push_str(text_delta);
                                        }
                                    }
                                    "response.output_image.done" => {
                                        // Image completed - embed as markdown
                                        if let Some(b64_json) = value["image"]["b64_json"].as_str() {
                                            content.push_str(&format!("\n![Generated Image](data:image/png;base64,{})\n", b64_json));
                                        } else if let Some(url) = value["image"]["url"].as_str() {
                                            content.push_str(&format!("\n![Generated Image]({})\n", url));
                                        }
                                    }
                                    "response.completed" => {
                                        // Stream completed
                                    }
                                    _ => {
                                        // Ignore unknown events
                                    }
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

