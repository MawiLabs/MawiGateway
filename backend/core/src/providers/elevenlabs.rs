use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::error::classify_response;
use crate::providers::{ChatStream, ProviderAdapter};
use crate::types::{
    AudioTranscriptionRequest, ChatCompletionRequest, SpeechToSpeechRequest, TextToSpeechRequest,
};

const PROVIDER: &str = "elevenlabs";

/// Default ElevenLabs voice when caller doesn't specify one ("Rachel").
/// Kept as a const so tests can reference the same default the runtime uses.
const DEFAULT_VOICE_ID: &str = "21m00Tcm4TlvDq8ikWAM";

pub struct ElevenLabsAdapter {
    client: Client,
    api_key: String,
}

impl ElevenLabsAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self { client, api_key }
    }
}

#[async_trait]
impl ProviderAdapter for ElevenLabsAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream> {
        Err(anyhow::anyhow!("Chat not supported by ElevenLabs"))
    }

    async fn text_to_speech(&self, req: &TextToSpeechRequest) -> Result<(String, Vec<u8>)> {
        // Voice on the URL — falls back to Rachel when blank so the playground
        // can leave the field empty for "use the provider's default".
        let voice = if req.voice.is_empty() {
            DEFAULT_VOICE_ID
        } else {
            req.voice.as_str()
        };
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice);

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&json!({
                "text": req.input,
                "model_id": req.model,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        let bytes = response.bytes().await?.to_vec();
        // ElevenLabs defaults to mp3 unless output_format overrides it. We
        // currently don't expose the override, so mp3 is correct.
        Ok(("audio/mpeg".to_string(), bytes))
    }

    async fn transcribe_audio(
        &self,
        audio_data: &[u8],
        req: &AudioTranscriptionRequest,
    ) -> Result<String> {
        let url = "https://api.elevenlabs.io/v1/speech-to-text";

        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.webm")
                    .mime_str("audio/webm")?,
            )
            .text("model_id", req.model.clone());

        let response = self
            .client
            .post(url)
            .header("xi-api-key", &self.api_key)
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
            .ok_or_else(|| anyhow::anyhow!("No text field in ElevenLabs STT response"))?
            .to_string();

        Ok(text)
    }

    async fn speech_to_speech(
        &self,
        audio_data: &[u8],
        req: &SpeechToSpeechRequest,
    ) -> Result<Vec<u8>> {
        let voice_id = req.voice.as_deref().unwrap_or(DEFAULT_VOICE_ID);
        let url = format!("https://api.elevenlabs.io/v1/speech-to-speech/{}", voice_id);

        let form = reqwest::multipart::Form::new()
            .part(
                "audio",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.mp3")
                    .mime_str("audio/mpeg")?,
            )
            .text("model_id", req.model.clone());

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        Ok(response.bytes().await?.to_vec())
    }
}
