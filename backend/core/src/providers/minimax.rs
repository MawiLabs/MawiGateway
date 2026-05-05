use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{ChatCompletionRequest, VideoGenerationRequest, VideoGenerationResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;

const PROVIDER: &str = "minimax";

/// MiniMax Hailuo video generation + chat.
/// API base: https://api.minimax.chat/v1
/// Video: POST /video_generation → task_id → poll GET /query/video_generation?task_id=...
/// → once "Success", fetch /files/retrieve?file_id=... for the download URL.
pub struct MiniMaxAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl MiniMaxAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.minimax.chat/v1".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for MiniMaxAdapter {
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error> {
        // MiniMax exposes an OpenAI-compatible /text/chatcompletion_v2 endpoint.
        let response = self
            .client
            .post(format!("{}/text/chatcompletion_v2", self.base_url))
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
                            if let Some(t) = value["choices"][0]["delta"]["content"].as_str() {
                                content.push_str(t);
                            }
                        }
                    }
                    content
                })
        });
        Ok(Box::pin(parsed_stream))
    }

    async fn generate_video(
        &self,
        req: &VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, anyhow::Error> {
        let body = json!({
            "model": req.model,
            "prompt": req.prompt,
        });

        let response = self
            .client
            .post(format!("{}/video_generation", self.base_url))
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
        let task_id = json["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No task_id in MiniMax response"))?;

        Ok(VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", task_id)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(&self, task_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        // Step 1 — query task status.
        let status_resp = self
            .client
            .get(format!(
                "{}/query/video_generation?task_id={}",
                self.base_url, task_id
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !status_resp.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, status_resp).await,
            ));
        }

        let status_json: serde_json::Value = status_resp.json().await?;
        let status = status_json["status"].as_str().unwrap_or("unknown");

        match status {
            "Success" | "Processing" | "Queueing" | "Preparing" => {
                if status != "Success" {
                    return Ok(json!({ "status": "processing" }));
                }
                // Step 2 — resolve file_id → download URL.
                let file_id = status_json["file_id"].as_str().unwrap_or("");
                let file_resp = self
                    .client
                    .get(format!(
                        "{}/files/retrieve?file_id={}",
                        self.base_url, file_id
                    ))
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .send()
                    .await?;
                let file_json: serde_json::Value = file_resp.json().await?;
                let video_url = file_json["file"]["download_url"].as_str().unwrap_or("");
                Ok(json!({ "status": "succeeded", "video_url": video_url }))
            }
            "Fail" => Ok(json!({
                "status": "failed",
                "error": status_json["base_resp"]["status_msg"].as_str().unwrap_or("unknown")
            })),
            _ => Ok(json!({ "status": "processing" })),
        }
    }
}
