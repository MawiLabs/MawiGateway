use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{ChatCompletionRequest, VideoGenerationRequest, VideoGenerationResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const PROVIDER: &str = "pika";

/// Pika Labs video generation.
/// API base: https://api.pika.art/v1 (developer endpoint).
/// Async: POST /generate → job_id → poll GET /jobs/{job_id}.
pub struct PikaAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl PikaAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.pika.art/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for PikaAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        anyhow::bail!("Pika is a video-only provider; chat is not supported")
    }

    async fn generate_video(
        &self,
        req: &VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, anyhow::Error> {
        let aspect_ratio = match req.size.as_deref() {
            Some("1920x1080") | Some("1280x720") => "16:9",
            Some("1080x1920") | Some("720x1280") => "9:16",
            Some("1024x1024") => "1:1",
            _ => "16:9",
        };

        let body = json!({
            "model": req.model,
            "prompt": req.prompt,
            "aspectRatio": aspect_ratio,
            "duration": req.duration.unwrap_or(5),
        });

        let response = self
            .client
            .post(format!("{}/generate", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let json: serde_json::Value = response.json().await?;
        let job_id = json["job_id"]
            .as_str()
            .or_else(|| json["id"].as_str())
            .ok_or_else(|| anyhow::anyhow!("No job_id in Pika response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", job_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, job_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        let response = self
            .client
            .get(format!("{}/jobs/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let job: serde_json::Value = response.json().await?;
        match job["status"].as_str().unwrap_or("unknown") {
            "finished" | "completed" | "succeeded" => {
                let video_url = job["videos"][0]["url"]
                    .as_str()
                    .or_else(|| job["video_url"].as_str())
                    .unwrap_or("");
                Ok(json!({ "status": "succeeded", "video_url": video_url }))
            }
            "failed" | "error" => Ok(json!({
                "status": "failed",
                "error": job["error"].as_str().unwrap_or("unknown")
            })),
            _ => Ok(json!({ "status": "processing" })),
        }
    }
}
