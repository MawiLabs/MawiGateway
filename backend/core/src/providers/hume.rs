use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::error::classify_response;
use crate::providers::{ChatStream, ProviderAdapter};
use crate::types::{
    AudioTranscriptionRequest, ChatCompletionRequest, SpeechToSpeechRequest, TextToSpeechRequest,
};

const PROVIDER: &str = "hume";

/// Hume AI — Octave TTS + EVI (Empathic Voice Interface).
/// API base: https://api.hume.ai/v0
/// Auth: X-Hume-Api-Key header.
///
/// Surface mapped to ProviderAdapter:
///   - text_to_speech → POST /tts (Octave) returns base64 audio JSON
///   - speech_to_speech → EVI conversational chat (best-effort REST shim;
///     full bidirectional EVI is WebSocket-only and lives outside this trait).
pub struct HumeAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl HumeAdapter {
    pub fn new(client: Client, api_key: String) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.hume.ai/v0".to_string(),
        }
    }
}

#[async_trait]
impl ProviderAdapter for HumeAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream> {
        anyhow::bail!("Hume AI is an audio provider; chat is not supported")
    }

    async fn text_to_speech(&self, req: &TextToSpeechRequest) -> Result<(String, Vec<u8>)> {
        // Hume Octave TTS. `voice` may be a Hume voice name (e.g. "ITO") or a
        // voice id; we pass it through unchanged. Empty voice → server default.
        let mut utterance = json!({ "text": req.input });
        if !req.voice.is_empty() {
            utterance["voice"] = json!({ "name": req.voice });
        }

        let body = json!({
            "utterances": [utterance],
            "format": { "type": "mp3" },
        });

        let response = self
            .client
            .post(format!("{}/tts", self.base_url))
            .header("X-Hume-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        // /tts returns JSON with generations[].audio as base64.
        let json: serde_json::Value = response.json().await?;
        let b64 = json["generations"][0]["audio"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No audio in Hume TTS response"))?;

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| anyhow::anyhow!("Hume TTS base64 decode failed: {}", e))?;

        Ok(("audio/mpeg".to_string(), bytes))
    }

    async fn transcribe_audio(
        &self,
        _audio_data: &[u8],
        _req: &AudioTranscriptionRequest,
    ) -> Result<String> {
        // Hume's transcription is bundled inside EVI / expression-measurement,
        // not exposed as a standalone STT endpoint. Reject explicitly so the
        // routing layer can fall through to a real STT provider.
        anyhow::bail!("Hume AI does not expose a standalone transcription endpoint")
    }

    async fn speech_to_speech(
        &self,
        audio_data: &[u8],
        req: &SpeechToSpeechRequest,
    ) -> Result<Vec<u8>> {
        // Best-effort REST shim around EVI's chat endpoint. EVI is primarily
        // a streaming WebSocket protocol — this REST hop sends one user audio
        // turn and returns the assistant's audio response. Multi-turn or
        // emotion-aware streaming should use the WebSocket directly.
        let voice_payload = req
            .voice
            .as_deref()
            .map(|v| json!({ "name": v }))
            .unwrap_or(serde_json::Value::Null);

        let form = reqwest::multipart::Form::new()
            .text("config_id", req.model.clone())
            .text("output_format", "mp3")
            .text("voice", voice_payload.to_string())
            .part(
                "audio",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.webm")
                    .mime_str("audio/webm")?,
            );

        let response = self
            .client
            .post(format!("{}/evi/chat", self.base_url))
            .header("X-Hume-Api-Key", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(classify_response(PROVIDER, response).await));
        }

        Ok(response.bytes().await?.to_vec())
    }
}
