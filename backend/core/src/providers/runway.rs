use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{ChatCompletionRequest, VideoGenerationRequest, VideoGenerationResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const PROVIDER: &str = "runway";

/// Runway Gen-4 / Gen-4.5 video generation.
/// API base: https://api.dev.runwayml.com/v1 (developer endpoint).
/// Uses async task model: POST /image_to_video or /text_to_video → task id → poll GET /tasks/{id}.
pub struct RunwayAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl RunwayAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.dev.runwayml.com/v1".to_string(),
        }
    }

    fn auth_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Runway-Version", "2024-11-06")
            .header("Content-Type", "application/json")
    }
}

#[async_trait]
impl ProviderAdapter for RunwayAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        anyhow::bail!("Runway is a video-only provider; chat is not supported")
    }

    async fn generate_video(
        &self,
        req: &VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, anyhow::Error> {
        // Runway expects "ratio" rather than pixel size, and integer seconds (5 or 10 typical for Gen-4).
        let ratio = match req.size.as_deref() {
            Some("1920x1080") | Some("1920:1080") => "1920:1080",
            Some("1080x1920") | Some("1080:1920") => "1080:1920",
            Some("1280x720") | Some("1280:720") => "1280:720",
            Some("720x1280") | Some("720:1280") => "720:1280",
            Some(other) => other,
            None => "1280:720",
        };
        let duration = req.duration.unwrap_or(5);

        let body = json!({
            "model": req.model,
            "promptText": req.prompt,
            "ratio": ratio,
            "duration": duration,
        });

        let url = format!("{}/text_to_video", self.base_url);
        let response = self
            .auth_headers(self.client.post(&url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let json: serde_json::Value = response.json().await?;
        let task_id = json["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No task id in Runway response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", task_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, task_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        let url = format!("{}/tasks/{}", self.base_url, task_id);
        let response = self.auth_headers(self.client.get(&url)).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let task: serde_json::Value = response.json().await?;
        match task["status"].as_str().unwrap_or("unknown") {
            "SUCCEEDED" => {
                let video_url = task["output"][0].as_str().unwrap_or("");
                Ok(json!({ "status": "succeeded", "video_url": video_url }))
            }
            "FAILED" | "CANCELLED" => Ok(json!({
                "status": "failed",
                "error": task["failure"].as_str().unwrap_or("unknown")
            })),
            _ => Ok(json!({ "status": "processing" })),
        }
    }
}
