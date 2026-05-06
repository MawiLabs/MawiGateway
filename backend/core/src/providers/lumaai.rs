use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{ChatCompletionRequest, VideoGenerationRequest, VideoGenerationResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const PROVIDER: &str = "lumaai";

/// Luma AI Dream Machine video generation.
/// API base: https://api.lumalabs.ai/dream-machine/v1
/// Async: POST /generations → id → poll GET /generations/{id}.
pub struct LumaAiAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl LumaAiAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.lumalabs.ai/dream-machine/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for LumaAiAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        anyhow::bail!("Luma AI is a video-only provider; chat is not supported")
    }

    async fn generate_video(
        &self,
        req: &VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, anyhow::Error> {
        let aspect_ratio = match req.size.as_deref() {
            Some("1920x1080") | Some("1280x720") => "16:9",
            Some("1080x1920") | Some("720x1280") => "9:16",
            Some("1024x1024") => "1:1",
            Some("4096x2160") => "21:9",
            _ => "16:9",
        };

        let body = json!({
            "model": req.model,
            "prompt": req.prompt,
            "aspect_ratio": aspect_ratio,
            "duration": format!("{}s", req.duration.unwrap_or(5)),
        });

        let response = self
            .client
            .post(format!("{}/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let id = json["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No id in Luma AI response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, id: &str) -> Result<serde_json::Value, anyhow::Error> {
        let response = self
            .client
            .get(format!("{}/generations/{}", self.base_url, id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let gen: serde_json::Value = response.json().await?;
        match gen["state"].as_str().unwrap_or("unknown") {
            "completed" => {
                let video_url = gen["assets"]["video"].as_str().unwrap_or("");
                Ok(json!({ "status": "succeeded", "video_url": video_url }))
            }
            "failed" => Ok(json!({
                "status": "failed",
                "error": gen["failure_reason"].as_str().unwrap_or("unknown")
            })),
            _ => Ok(json!({ "status": "processing" })),
        }
    }
}
