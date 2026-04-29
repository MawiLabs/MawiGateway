use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{ChatCompletionRequest, VideoGenerationRequest, VideoGenerationResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

const PROVIDER: &str = "bytedance";

/// ByteDance Seedance video generation (via Volcano Engine Ark).
/// API base: https://ark.cn-beijing.volces.com/api/v3
/// Async: POST /contents/generations/tasks → id → poll GET /contents/generations/tasks/{id}.
pub struct ByteDanceAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ByteDanceAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for ByteDanceAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        anyhow::bail!("ByteDance Seedance is a video-only provider; chat is not supported")
    }

    async fn generate_video(
        &self,
        req: &VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, anyhow::Error> {
        // Seedance encodes resolution + ratio + duration as parameter tags inside the prompt content.
        let ratio = match req.size.as_deref() {
            Some("1920x1080") | Some("1280x720") => "16:9",
            Some("1080x1920") | Some("720x1280") => "9:16",
            Some("1024x1024") => "1:1",
            _ => "16:9",
        };
        let duration = req.duration.unwrap_or(5);
        let resolution = match req.size.as_deref() {
            Some("1920x1080") | Some("1080x1920") => "1080p",
            Some("3840x2160") | Some("2160x3840") => "4k",
            _ => "720p",
        };

        let prompt_with_tags = format!(
            "{} --resolution {} --ratio {} --duration {}",
            req.prompt, resolution, ratio, duration
        );

        let body = json!({
            "model": req.model,
            "content": [
                { "type": "text", "text": prompt_with_tags }
            ]
        });

        let response = self
            .client
            .post(format!("{}/contents/generations/tasks", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let json: serde_json::Value = response.json().await?;
        let task_id = json["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No id in ByteDance Seedance response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", task_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, task_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        let response = self
            .client
            .get(format!(
                "{}/contents/generations/tasks/{}",
                self.base_url, task_id
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        let task: serde_json::Value = response.json().await?;
        match task["status"].as_str().unwrap_or("unknown") {
            "succeeded" => {
                let video_url = task["content"]["video_url"].as_str().unwrap_or("");
                Ok(json!({ "status": "succeeded", "video_url": video_url }))
            }
            "failed" | "cancelled" => Ok(json!({
                "status": "failed",
                "error": task["error"]["message"].as_str().unwrap_or("unknown")
            })),
            _ => Ok(json!({ "status": "processing" })),
        }
    }
}
