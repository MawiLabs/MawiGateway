use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;
use crate::types::ChatCompletionRequest;
use super::{ProviderAdapter, ChatStream};

pub struct GeminiAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // Convert OpenAI format to Gemini format
        let contents = req.messages.iter().map(|msg| {
            json!({
                "role": if msg.role == "assistant" { "model" } else { "user" },
                "parts": [{"text": msg.content}]
            })
        }).collect::<Vec<_>>();

        // Use the model name as-is (e.g., "gemini-2.0-flash")
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            req.model, self.api_key
        );

        let response = self.client
            .post(&url)
            .json(&json!({
                "contents": contents,
            }))
            .send()
            .await?;

        let stream = response.bytes_stream();
        
        let parsed_stream = stream.map(|chunk_result| {
            chunk_result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    eprintln!("Gemini raw chunk: '{}'", text);
                    let mut content = String::new();
                    
                    // Gemini streams JSON objects separated by newlines
                    for line in text.lines() {
                        let line = line.trim();
                        eprintln!("  Line: '{}'", line);
                        if line.is_empty() {
                            continue;
                        }
                        
                        // Parse the JSON response
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                            eprintln!("  Parsed JSON: {:?}", value);
                            if let Some(text_content) = value["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                eprintln!("  Extracted content: '{}'", text_content);
                                content.push_str(text_content);
                            }
                        }
                    }
                    eprintln!("  Chunk total content: '{}' ({} bytes)", content, content.len());
                    Ok(content)
                })
        }); // REMOVED .filter() to see all debug output

        Ok(Box::pin(parsed_stream))
    }

    async fn generate_video(&self, req: &crate::types::VideoGenerationRequest) -> Result<crate::types::VideoGenerationResponse, anyhow::Error> {
        use serde_json::json;
        
        eprintln!("🎬 Google Veo 3 video generation - model: {}, prompt: {}", req.model, req.prompt);
        
        // Veo 3 uses predictLongRunning endpoint
        let response = self.client
            .post(format!("{}/models/{}:predictLongRunning?key={}", self.base_url, req.model, self.api_key))
            .json(&json!({
                "instances": [{
                    "prompt": req.prompt
                }]
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Google Veo API error {}: {}", status, error_text));
        }

        let json: serde_json::Value = response.json().await?;
        eprintln!("📥 Veo 3 response: {}", serde_json::to_string_pretty(&json)?);
        
        // Extract operation name
        let operation_name = json["name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No operation name in response"))?;
        
        eprintln!("🔄 Operation created: {} - returning immediately", operation_name);
        
        Ok(crate::types::VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", operation_name)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, operation_name: &str) -> Result<serde_json::Value, anyhow::Error> {
        // Poll operation status
        let poll_url = format!("{}/{}?key={}", self.base_url, operation_name, self.api_key);
        
        let response = self.client
            .get(&poll_url)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to poll operation: {}", error_text);
        }
        
        let operation: serde_json::Value = response.json().await?;
        let is_done = operation["done"].as_bool().unwrap_or(false);
        
        // If done, extract video URI
        if is_done {
            let video_uri = operation["response"]["generateVideoResponse"]["generatedSamples"][0]["video"]["uri"]
                .as_str()
                .unwrap_or("");
            
            return Ok(serde_json::json!({
                "status": "succeeded",
                "video_url": video_uri
            }));
        }
        
        Ok(serde_json::json!({
            "status": "processing"
        }))
    }

    async fn get_video_content(&self, video_uri: &str) -> Result<Vec<u8>, anyhow::Error> {
        // Download video with API key
        let response = self.client
            .get(format!("{}?key={}", video_uri, self.api_key))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to fetch video: {}", error_text);
        }
        
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn generate_image(&self, req: &crate::types::ImageGenerationRequest) -> Result<crate::types::ImageGenerationResponse, anyhow::Error> {
        use serde_json::json;
        
        eprintln!("🎨 Gemini image generation - model: {}, prompt: {}", req.model, req.prompt);
        
        let response = self.client
            .post(format!("{}/models/{}:generateContent?key={}", self.base_url, req.model, self.api_key))
            .json(&json!({
                "contents": [{
                    "parts": [{
                        "text": req.prompt
                    }]
                }]
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Gemini image API error {}: {}", status, error_text));
        }

        let json: serde_json::Value = response.json().await?;
        eprintln!("📥 Gemini image response received");
        
        // Extract base64 image from response
        // Gemini returns: candidates[0].content.parts[0].inline_data.data
        let image_data = json["candidates"][0]["content"]["parts"][0]["inline_data"]["data"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No image data in response"))?;
        
        Ok(crate::types::ImageGenerationResponse {
            created: chrono::Utc::now().timestamp() as u64,
            data: vec![crate::types::ImageData {
                b64_json: Some(image_data.to_string()),
                url: None,
                revised_prompt: None,
            }],
        })
    }
}
