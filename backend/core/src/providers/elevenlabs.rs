use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

use crate::providers::{ProviderAdapter, ChatStream};
use crate::types::{ChatCompletionRequest, TextToSpeechRequest, AudioTranscriptionRequest, SpeechToSpeechRequest};

pub struct ElevenLabsAdapter {
    client: Client,
    api_key: String,
}

impl ElevenLabsAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
        }
    }
}

#[async_trait]
impl ProviderAdapter for ElevenLabsAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream> {
        Err(anyhow::anyhow!("Chat not supported by ElevenLabs"))
    }

    async fn text_to_speech(&self, req: &TextToSpeechRequest) -> Result<(String, Vec<u8>)> {
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}",
            req.voice
        );

        eprintln!("🔊 ElevenLabs TTS request to: {}", url);

        let request_body = json!({
            "text": req.input,
            "model_id": req.model,
        });

        let response = self.client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!("❌ ElevenLabs API error: {}", error_text);
            anyhow::bail!("ElevenLabs API error {}: {}", status, error_text);
        }

        let bytes = response.bytes().await?.to_vec();
        // Default to mp3 as per curl example (unless output_format is specified, which defaults to mp3)
        Ok(("audio/mpeg".to_string(), bytes))
    }

    async fn transcribe_audio(&self, audio_data: &[u8], req: &AudioTranscriptionRequest) -> Result<String> {
        let url = "https://api.elevenlabs.io/v1/speech-to-text";
        
        eprintln!("🎤 ElevenLabs STT request to: {}", url);

        // Build multipart form
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(audio_data.to_vec())
                .file_name("audio.webm")
                .mime_str("audio/webm")?)
            .text("model_id", req.model.clone());

        let response = self.client
            .post(url)
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!("❌ ElevenLabs STT error: {}", error_text);
            anyhow::bail!("ElevenLabs STT error {}: {}", status, error_text);
        }

        // Response is JSON: {"text": "transcribed text"}
        let json: serde_json::Value = response.json().await?;
        let text = json["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No text field in STT response"))?
            .to_string();

        Ok(text)
    }

    async fn speech_to_speech(&self, audio_data: &[u8], req: &SpeechToSpeechRequest) -> Result<Vec<u8>> {
        // Use default voice if not provided
        let voice_id = req.voice.as_deref().unwrap_or("21m00Tcm4TlvDq8ikWAM");
        let url = format!(
            "https://api.elevenlabs.io/v1/speech-to-speech/{}",
            voice_id
        );

        eprintln!("🔄 ElevenLabs STS request to: {}", url);

        // Build multipart form
        let form = reqwest::multipart::Form::new()
            .part("audio", reqwest::multipart::Part::bytes(audio_data.to_vec())
                .file_name("audio.mp3")
                .mime_str("audio/mpeg")?)
            .text("model_id", req.model.clone());

        let response = self.client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            eprintln!("❌ ElevenLabs STS error: {}", error_text);
            anyhow::bail!("ElevenLabs STS error {}: {}", status, error_text);
        }

        let bytes = response.bytes().await?.to_vec();
        Ok(bytes)
    }
}
