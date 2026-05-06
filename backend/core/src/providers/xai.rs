use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{
    ChatCompletionRequest, ImageGenerationRequest, ImageGenerationResponse, VideoGenerationRequest,
    VideoGenerationResponse,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;

const PROVIDER: &str = "xai";

pub struct XaiAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl XaiAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.x.ai/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for XaiAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": req.model,
                "messages": req.messages,
                "stream": true,
                "temperature": req.temperature,
                "max_tokens": req.max_tokens,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
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
                            if let Some(text_content) =
                                value["choices"][0]["delta"]["content"].as_str()
                            {
                                content.push_str(text_content);
                            }
                        }
                    }
                    content
                })
        });

        Ok(Box::pin(parsed_stream))
    }

    /// Grok Imagine — image generation (OpenAI-compatible endpoint).
    async fn generate_image(
        &self,
        req: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, anyhow::Error> {
        let response = self
            .client
            .post(format!("{}/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": req.model,
                "prompt": req.prompt,
                "n": req.n,
                "response_format": "b64_json",
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let data = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No image data in xAI response"))?;

        let images = data
            .iter()
            .map(|item| crate::types::ImageData {
                url: item["url"].as_str().map(|s| s.to_string()),
                b64_json: item["b64_json"].as_str().map(|s| s.to_string()),
                revised_prompt: item["revised_prompt"].as_str().map(|s| s.to_string()),
            })
            .collect();

        Ok(ImageGenerationResponse {
            created: chrono::Utc::now().timestamp() as u64,
            data: images,
        })
    }

    /// Grok Imagine — image-to-video / text-to-video.
    /// xAI's video API is async; we return a JOB_ID and clients poll.
    async fn generate_video(
        &self,
        req: &VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, anyhow::Error> {
        let duration = req.duration.unwrap_or(6);
        let size = req.size.clone().unwrap_or_else(|| "1280x720".to_string());

        let response = self
            .client
            .post(format!("{}/videos/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": req.model,
                "prompt": req.prompt,
                "duration": duration,
                "size": size,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let job_id = json["id"]
            .as_str()
            .or_else(|| json["job_id"].as_str())
            .ok_or_else(|| anyhow::anyhow!("No job id in xAI Imagine response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", job_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, job_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        let response = self
            .client
            .get(format!("{}/videos/generations/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let job: serde_json::Value = response.json().await?;
        let status = job["status"].as_str().unwrap_or("unknown");

        if status == "completed" || status == "succeeded" {
            let url = job["video_url"]
                .as_str()
                .or_else(|| job["url"].as_str())
                .or_else(|| job["data"][0]["url"].as_str())
                .unwrap_or("");
            return Ok(json!({ "status": "succeeded", "video_url": url }));
        }

        if status == "failed" {
            return Ok(json!({
                "status": "failed",
                "error": job["error"].as_str().unwrap_or("unknown error")
            }));
        }

        Ok(json!({ "status": "processing" }))
    }
}
