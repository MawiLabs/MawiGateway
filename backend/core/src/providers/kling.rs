use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{ChatCompletionRequest, VideoGenerationRequest, VideoGenerationResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const PROVIDER: &str = "kling";

/// Kuaishou Kling video generation.
/// API base: https://api.klingai.com/v1
/// Async: POST /videos/text2video → task_id → poll GET /videos/text2video/{task_id}.
/// Auth: JWT bearer; we accept the pre-signed JWT in `api_key`. (Signing the JWT
/// from access/secret pair belongs in the credential store, not at request time.)
pub struct KlingAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl KlingAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.klingai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for KlingAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        anyhow::bail!("Kling is a video-only provider; chat is not supported")
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
        let duration = req.duration.unwrap_or(5);

        let body = json!({
            "model_name": req.model,
            "prompt": req.prompt,
            "duration": duration.to_string(),
            "aspect_ratio": aspect_ratio,
            "mode": "std",
        });

        let response = self
            .client
            .post(format!("{}/videos/text2video", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let json: serde_json::Value = response.json().await?;
        let task_id = json["data"]["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No task_id in Kling response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", task_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, task_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        let response = self
            .client
            .get(format!("{}/videos/text2video/{}", self.base_url, task_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let task: serde_json::Value = response.json().await?;
        let status = task["data"]["task_status"].as_str().unwrap_or("unknown");

        match status {
            "succeed" => {
                let video_url = task["data"]["task_result"]["videos"][0]["url"]
                    .as_str()
                    .unwrap_or("");
                Ok(json!({ "status": "succeeded", "video_url": video_url }))
            }
            "failed" => Ok(json!({
                "status": "failed",
                "error": task["data"]["task_status_msg"].as_str().unwrap_or("unknown")
            })),
            _ => Ok(json!({ "status": "processing" })),
        }
    }
}
