use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

use crate::providers::{ProviderAdapter, ChatStream};
use crate::types::{ChatCompletionRequest, ImageGenerationRequest, ImageGenerationResponse};

pub struct AzureProvider {
    client: Client,
    api_key: String,
    base_url: String,
    api_version: String,
}

impl AzureProvider {
    pub fn new(client: Client, api_key: String, base_url: String, api_version: Option<String>) -> Self {
        Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_version: api_version.unwrap_or_else(|| "2024-12-01-preview".to_string()),
        }
    }
}

#[async_trait]
impl ProviderAdapter for AzureProvider {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream> {
        
        // Azure requires deployment name as the model identifier
        // API format: {base_url}/openai/deployments/{deployment}/chat/completions?api-version={version}
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.base_url,
            req.model,
            self.api_version
        );

        eprintln!("🔵 Azure request to: {}", url);

        let request_body = json!({
            "messages": req.messages,
            "max_tokens": req.max_tokens.unwrap_or(1000),
            "temperature": req.temperature.unwrap_or(0.7),
            "stream": true,
            "reasoning_effort": req.reasoning_effort,
        });

        eprintln!("📤 Request body: {}", serde_json::to_string_pretty(&request_body)?);

        let response = self.client
            .post(&url)
            .header("api-key", &self.api_key)  // Azure uses api-key header, not Bearer
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        eprintln!("📥 Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!("❌ Azure error response: {}", error_text);
            anyhow::bail!("Azure API error {}: {}", status, error_text);
        }

        // Stream SSE responses (same approach as OpenAI)
        let mut stream = response.bytes_stream();
        
        let content_stream = Box::pin(async_stream::stream! {
            use tokio_stream::StreamExt;
            let mut buffer = String::new();
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);
                        
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();
                            
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    return;
                                }
                                
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                        yield Ok(content.to_string());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Stream error: {}", e);
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        return;
                    }
                }
            }
        });

        Ok(content_stream)
    }

    async fn generate_image(&self, req: &ImageGenerationRequest) -> Result<ImageGenerationResponse, anyhow::Error> {
        // DALL-E 3 on Azure
        // API format: {base_url}/openai/deployments/{deployment}/images/generations?api-version={version}
        
        // Use configured version or default to DALL-E 3 standard
        let api_version = if self.api_version.is_empty() || self.api_version == "2024-12-01-preview" {
             "2024-02-15-preview" // Default specific for images if main is chat default
        } else {
             &self.api_version
        };

        // deployment name comes from req.model
        let url = format!(
            "{}/openai/deployments/{}/images/generations?api-version={}",
            self.base_url,
            req.model,
            api_version
        );

        eprintln!("🎨 Azure Image Gen REQUEST START");
        eprintln!("📍 URL: {}", url);
        eprintln!("🔑 Auth: Header present (len: {})", self.api_key.len());

        let request_body = json!({
            "prompt": req.prompt,
            "n": req.n,
            "size": req.size,
            "style": req.style,
            "quality": req.quality,
        });

        eprintln!("📤 Body: {}", serde_json::to_string_pretty(&request_body)?);

        let response = self.client
            .post(&url)
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        eprintln!("📥 Response Status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!("❌ Azure Image Gen ERROR BODY: {}", error_text);
            eprintln!("❌ FULL URL WAS: {}", url);
            anyhow::bail!("Azure Image API error {} at {}: {}", status, url, error_text);
        }

        let body_text = response.text().await?;
        eprintln!("✅ Azure Image Gen SUCCESS BODY: {}", body_text);
        
        let image_response: ImageGenerationResponse = serde_json::from_str(&body_text)?;
        Ok(image_response)
    }

    async fn generate_video(&self, req: &crate::types::VideoGenerationRequest) -> Result<crate::types::VideoGenerationResponse> {
        eprintln!("🎬 Azure Sora video generation - deployment: {}, prompt: {}", req.model, req.prompt);

        // Azure Sora format: https://{resource}.api.cognitive.microsoft.com/openai/v1/video/generations/jobs?api-version=preview
        let url = format!(
            "{}/openai/v1/video/generations/jobs?api-version=preview",
            self.base_url
        );

        eprintln!("📍 Azure Sora URL: {}", url);

        // Parse size like "1280x720" into width and height
        let (width, height) = if let Some(size) = &req.size {
            let parts: Vec<&str> = size.split('x').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                ("1080".to_string(), "1080".to_string())
            }
        } else {
            ("1080".to_string(), "1080".to_string())
        };

        let request_body = json!({
            "model": req.model,
            "prompt": req.prompt,
            "height": height,
            "width": width,
            "n_seconds": req.duration.unwrap_or(5).to_string(),
            "n_variants": "1"
        });

        eprintln!("📤 Request body: {}", serde_json::to_string_pretty(&request_body)?);

        let response = self.client
            .post(&url)
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!("❌ Azure Video Gen error: {}", error_text);
            anyhow::bail!("Azure Video API error {} at {}: {}", status, url, error_text);
        }

        let json: serde_json::Value = response.json().await?;
        eprintln!("📥 Azure Sora response: {}", serde_json::to_string_pretty(&json)?);
        
        // Return job ID immediately - don't poll (polling will be done by frontend)
        let job_id = json["id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No job ID in response"))?;
        
        eprintln!("🔄 Job created: {} - returning immediately", job_id);
        
        // Return the job ID as the "URL" so frontend can poll it
        Ok(crate::types::VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", job_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, job_id: &str) -> Result<serde_json::Value> {
        // Poll Azure job status
        let poll_url = format!("{}/openai/v1/video/generations/jobs/{}?api-version=preview", self.base_url, job_id);
        
        let response = self.client
            .get(&poll_url)
            .header("api-key", &self.api_key)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to poll job: {}", error_text);
        }
        
        let job_status: serde_json::Value = response.json().await?;
        let status = job_status["status"].as_str().unwrap_or("unknown");
        
        // If succeeded, fetch video URL
        if status == "succeeded" {
            let generations = job_status["generations"].as_array();
            if let Some(gens) = generations {
                if let Some(first_gen) = gens.first() {
                    let generation_id = first_gen["id"].as_str().unwrap_or("");
                    
                    // Get video download URL
                    let video_url = format!("{}/openai/v1/video/generations/{}/content/video?api-version=preview", 
                        self.base_url, generation_id);
                    
                    return Ok(serde_json::json!({
                        "status": "succeeded",
                        "video_url": video_url
                    }));
                }
            }
        }
        
        // Return current status
        Ok(serde_json::json!({
            "status": status
        }))
    }

    async fn get_video_content(&self, generation_id: &str) -> Result<Vec<u8>> {
        let video_url = format!("{}/openai/v1/video/generations/{}/content/video?api-version=preview", 
            self.base_url, generation_id);
        
        let response = self.client
            .get(&video_url)
            .header("api-key", &self.api_key)
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
