use super::{ChatStream, ProviderAdapter};
use crate::types::ChatCompletionRequest;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio_stream::StreamExt;

/// Wrap raw signed-16-bit PCM samples in a minimal RIFF/WAV header so
/// downstream callers (browsers, ffmpeg, the ViralStory mixer) can
/// treat the bytes as a real audio file. Gemini TTS returns raw PCM
/// at 24kHz mono; this is the smallest correct WAV envelope around it.
fn pcm16_to_wav(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let byte_rate = sample_rate * (channels as u32) * 2;
    let block_align: u16 = channels * 2;
    let data_size = pcm.len() as u32;
    let riff_size = 36u32 + data_size;

    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes());  // audio format = PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

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
        let contents = req
            .messages
            .iter()
            .map(|msg| {
                json!({
                    "role": if msg.role == "assistant" { "model" } else { "user" },
                    "parts": [{"text": msg.content}]
                })
            })
            .collect::<Vec<_>>();

        // Use the model name as-is (e.g., "gemini-2.0-flash")
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            req.model, self.api_key
        );

        let response = self
            .client
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
                .map(|bytes| {
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
                            if let Some(text_content) =
                                value["candidates"][0]["content"]["parts"][0]["text"].as_str()
                            {
                                eprintln!("  Extracted content: '{}'", text_content);
                                content.push_str(text_content);
                            }
                        }
                    }
                    eprintln!(
                        "  Chunk total content: '{}' ({} bytes)",
                        content,
                        content.len()
                    );
                    content
                })
        }); // REMOVED .filter() to see all debug output

        Ok(Box::pin(parsed_stream))
    }

    async fn generate_video(
        &self,
        req: &crate::types::VideoGenerationRequest,
    ) -> Result<crate::types::VideoGenerationResponse, anyhow::Error> {
        use serde_json::json;

        eprintln!(
            "🎬 Google Veo 3 video generation - model: {}, prompt: {}",
            req.model, req.prompt
        );

        // Veo 3 uses predictLongRunning endpoint
        let response = self
            .client
            .post(format!(
                "{}/models/{}:predictLongRunning?key={}",
                self.base_url, req.model, self.api_key
            ))
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
            return Err(anyhow::anyhow!(
                "Google Veo API error {}: {}",
                status,
                error_text
            ));
        }

        let json: serde_json::Value = response.json().await?;
        eprintln!(
            "📥 Veo 3 response: {}",
            serde_json::to_string_pretty(&json)?
        );

        // Extract operation name
        let operation_name = json["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No operation name in response"))?;

        eprintln!(
            "🔄 Operation created: {} - returning immediately",
            operation_name
        );

        Ok(crate::types::VideoGenerationResponse {
            url: Some(format!("JOB_ID:{}", operation_name)),
            data: None,
            format: "mp4".to_string(),
        })
    }

    async fn poll_video_job(
        &self,
        operation_name: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        // Poll operation status
        let poll_url = format!("{}/{}?key={}", self.base_url, operation_name, self.api_key);

        let response = self.client.get(&poll_url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to poll operation: {}", error_text);
        }

        let operation: serde_json::Value = response.json().await?;
        let is_done = operation["done"].as_bool().unwrap_or(false);

        // If done, extract video URI
        if is_done {
            let video_uri = operation["response"]["generateVideoResponse"]["generatedSamples"][0]
                ["video"]["uri"]
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
        let response = self
            .client
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

    async fn text_to_speech(
        &self,
        req: &crate::types::TextToSpeechRequest,
    ) -> Result<(String, Vec<u8>), anyhow::Error> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        use serde_json::json;

        eprintln!(
            "🔊 Gemini TTS — model: {}, voice: {}, len: {}",
            req.model,
            req.voice,
            req.input.len()
        );

        // Gemini TTS prebuilt voice names. We accept the OpenAI-style
        // names (alloy/echo/...) for drop-in compatibility and map them
        // to Gemini's catalogue. Anything else falls through unchanged
        // so callers can pick any voice from
        // https://ai.google.dev/gemini-api/docs/speech-generation
        let voice = match req.voice.to_lowercase().as_str() {
            // OpenAI-shape aliases → reasonable Gemini equivalents
            "alloy" => "Aoede",        // breezy, neutral
            "echo" => "Charon",        // informative, masc
            "fable" => "Puck",         // upbeat, expressive
            "onyx" => "Orus",          // firm, low pitch
            "nova" => "Leda",          // youthful, bright
            "shimmer" => "Sulafat",    // warm
            "" | "default" => "Kore",  // Gemini's safe default
            _ => req.voice.as_str(),
        };

        let response = self
            .client
            .post(format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url, req.model, self.api_key
            ))
            .json(&json!({
                "contents": [{
                    "parts": [{ "text": req.input }]
                }],
                "generationConfig": {
                    "responseModalities": ["AUDIO"],
                    "speechConfig": {
                        "voiceConfig": {
                            "prebuiltVoiceConfig": { "voiceName": voice }
                        }
                    }
                }
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Gemini TTS API error {}: {}",
                status,
                error_text
            ));
        }

        let json: serde_json::Value = response.json().await?;

        // Gemini returns inline_data with base64 audio. Modality is
        // typically "audio/L16;rate=24000;codec=pcm" — raw 24kHz PCM.
        let part = &json["candidates"][0]["content"]["parts"][0]["inlineData"];
        let b64 = part["data"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Gemini TTS returned no audio data"))?;
        let mime = part["mimeType"]
            .as_str()
            .unwrap_or("audio/L16;rate=24000;codec=pcm")
            .to_string();
        let bytes = BASE64
            .decode(b64)
            .map_err(|e| anyhow::anyhow!("base64 decode failed: {}", e))?;

        // If the response is raw PCM, wrap it in a minimal WAV header so
        // the browser <audio> tag and downstream encoders accept it as
        // a real audio file. Gemini's "audio/L16;rate=24000;codec=pcm"
        // means signed 16-bit, mono, little-endian.
        let (content_type, payload) = if mime.contains("L16") || mime.contains("pcm") {
            let sample_rate: u32 = mime
                .split(';')
                .find_map(|s| s.trim().strip_prefix("rate="))
                .and_then(|n| n.parse().ok())
                .unwrap_or(24_000);
            let wav = pcm16_to_wav(&bytes, sample_rate, 1);
            ("audio/wav".to_string(), wav)
        } else {
            (mime, bytes)
        };

        Ok((content_type, payload))
    }

    async fn generate_image(
        &self,
        req: &crate::types::ImageGenerationRequest,
    ) -> Result<crate::types::ImageGenerationResponse, anyhow::Error> {
        use serde_json::json;

        eprintln!(
            "🎨 Gemini image generation - model: {}, prompt: {}",
            req.model, req.prompt
        );

        let response = self
            .client
            .post(format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url, req.model, self.api_key
            ))
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
            return Err(anyhow::anyhow!(
                "Gemini image API error {}: {}",
                status,
                error_text
            ));
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
