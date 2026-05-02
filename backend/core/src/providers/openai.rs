use super::{ChatStream, ProviderAdapter};
use crate::error::classify_response;
use crate::types::{AudioTranscriptionRequest, ChatCompletionRequest, TextToSpeechRequest};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;

/// Identifier used in ProviderError + metric labels for this adapter.
const PROVIDER: &str = "openai";

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
        let is_reasoning_model = req.model.starts_with("o1")
            || req.model.starts_with("o3")
            || req.model.contains("gpt-5");

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

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        // Surface non-2xx as typed ProviderError so the executor's failover
        // gate can distinguish 429/5xx (retryable, try next model) from
        // 4xx (non-retryable, fail fast). Without this, streaming returned
        // a "successful" empty stream on errors and silently masked rate
        // limits.
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
                            if let Some(text_content) =
                                value["choices"][0]["delta"]["content"].as_str()
                            {
                                content.push_str(text_content);
                            }
                        }
                    }
                    content
                })
        }); // REMOVED .filter() to see all debug output

        Ok(Box::pin(parsed_stream))
    }

    async fn generate_image(
        &self,
        req: &crate::types::ImageGenerationRequest,
    ) -> Result<crate::types::ImageGenerationResponse, anyhow::Error> {
        // DALL-E / gpt-image. Endpoint: POST /v1/images/generations.
        // Body shape mirrors OpenAI's published spec — model/prompt
        // required, n + size optional (defaults 1024x1024 if omitted).
        let mut body = serde_json::json!({
            "model": req.model,
            "prompt": req.prompt,
            "n": req.n,
            "size": req.size,
        });
        if let Some(q) = &req.quality {
            body.as_object_mut().unwrap().insert("quality".into(), serde_json::json!(q));
        }
        if let Some(s) = &req.style {
            body.as_object_mut().unwrap().insert("style".into(), serde_json::json!(s));
        }

        let response = self
            .client
            .post(format!("{}/images/generations", self.base_url))
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

        let img_response: crate::types::ImageGenerationResponse = response.json().await?;
        Ok(img_response)
    }

    async fn generate_video(
        &self,
        req: &crate::types::VideoGenerationRequest,
    ) -> Result<crate::types::VideoGenerationResponse, anyhow::Error> {
        #[cfg(debug_assertions)]
        eprintln!("🎬 OpenAI Sora video generation - model: {}", req.model);

        let size = req.size.clone().unwrap_or_else(|| "1280x720".to_string());
        // Sora 2 only accepts {4, 8, 12} seconds — anything else 400s
        // with `Invalid value: '5'. Supported values are: '4', '8',
        // and '12'.` Snap the caller's request to the nearest valid
        // value so the canvas can keep speaking in arbitrary seconds.
        let requested = req.duration.unwrap_or(8) as i32;
        let duration = [4_i32, 8, 12]
            .iter()
            .min_by_key(|&&v| (v - requested).abs())
            .copied()
            .unwrap_or(8)
            .to_string();

        // Sora 2's image-to-video path requires a previously-uploaded
        // file referenced as `{"type":"image","file_id":"file-…"}`,
        // NOT a raw multipart attachment under `input_reference` —
        // direct-attach returns
        //   "Invalid type for 'input_reference': expected an object,
        //    but got a file instead."
        // Threading the two-step (POST /v1/files → POST /v1/videos
        // with file_id) flow is its own change tracked separately.
        // Until that lands, fold any caller-supplied reference URL
        // into the prompt so the narrative context still reaches Sora.
        let prompt_with_ref = match req.input_image_url.as_deref() {
            Some(url) => format!("{}\n\nReference image: {}", req.prompt, url),
            None => req.prompt.clone(),
        };

        // OpenAI Sora uses multipart/form-data.
        let form = reqwest::multipart::Form::new()
            .text("prompt", prompt_with_ref)
            .text("model", req.model.clone())
            .text("size", size)
            .text("seconds", duration);

        let response = self
            .client
            .post(format!("{}/videos", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let json: serde_json::Value = response.json().await?;
        eprintln!("📥 OpenAI Sora response: [JSON hidden]");

        // Return video ID immediately
        let video_id = json["id"]
            .as_str()
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

        let response = self
            .client
            .get(&poll_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
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

        let response = self
            .client
            .get(&video_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// OpenAI Text-to-Speech (`tts-1`, `tts-1-hd`, `gpt-4o-mini-tts`).
    /// POST `/audio/speech` returns raw audio bytes (mp3 by default).
    async fn text_to_speech(
        &self,
        req: &TextToSpeechRequest,
    ) -> Result<(String, Vec<u8>), anyhow::Error> {
        // OpenAI TTS requires a voice — when caller leaves it blank we
        // pick `alloy` (the most neutral of the six built-in voices).
        let voice = if req.voice.is_empty() {
            "alloy"
        } else {
            req.voice.as_str()
        };

        let response = self
            .client
            .post(format!("{}/audio/speech", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": req.model,
                "input": req.input,
                "voice": voice,
                "response_format": "mp3",
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        Ok(("audio/mpeg".to_string(), response.bytes().await?.to_vec()))
    }

    /// OpenAI Whisper (`whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`).
    /// POST `/audio/transcriptions` is multipart (file + model + optional language).
    async fn transcribe_audio(
        &self,
        audio_data: &[u8],
        req: &AudioTranscriptionRequest,
    ) -> Result<String, anyhow::Error> {
        let mut form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.webm")
                    .mime_str("audio/webm")?,
            )
            .text("model", req.model.clone())
            .text("response_format", "json");

        if let Some(lang) = &req.language {
            if !lang.is_empty() {
                form = form.text("language", lang.clone());
            }
        }

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("OpenAI Whisper: no text field in response"))?
            .to_string();
        Ok(text)
    }
}

// Additional methods for OpenAIAdapter (not part of ProviderAdapter trait)
impl OpenAIAdapter {
    /// Stream from OpenAI /responses endpoint (GPT-5 multimodal)
    pub async fn stream_responses(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatStream, anyhow::Error> {
        #[cfg(debug_assertions)]
        eprintln!(
            "🌐 Using OpenAI /responses endpoint for multimodal model: {}",
            req.model
        );

        let body = json!({
            "model": req.model,
            "messages": req.messages,
            "stream": true,
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });

        let response = self
            .client
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
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
                                        if let Some(b64_json) = value["image"]["b64_json"].as_str()
                                        {
                                            content.push_str(&format!(
                                                "\n![Generated Image](data:image/png;base64,{})\n",
                                                b64_json
                                            ));
                                        } else if let Some(url) = value["image"]["url"].as_str() {
                                            content.push_str(&format!(
                                                "\n![Generated Image]({})\n",
                                                url
                                            ));
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
                    content
                })
        });

        Ok(Box::pin(parsed_stream))
    }
}
