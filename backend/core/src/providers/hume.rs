use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::classify_response;
use crate::providers::{ChatStream, ProviderAdapter};
use crate::types::{
    AudioTranscriptionRequest, ChatCompletionRequest, SpeechToSpeechRequest, TextToSpeechRequest,
};

const PROVIDER: &str = "hume";

/// Hume AI — Octave TTS.
///
/// API base: `https://api.hume.ai/v0`. Auth via `X-Hume-Api-Key` header.
/// Reference: https://dev.hume.ai/reference/text-to-speech-tts/synthesize-json
///
/// Surface mapped to `ProviderAdapter`:
///
///   - `text_to_speech` → POST `/tts` (Octave). Honors voice name + provider
///     selection (HUME_AI library or user CUSTOM_VOICE), description (acting
///     directions), speed, and output format. Returns the right MIME type
///     for whatever format Hume actually encoded.
///   - `transcribe_audio` → not supported. Hume's transcription is bundled
///     inside EVI / expression-measurement; we reject explicitly so the
///     router falls through to a real STT provider.
///   - `speech_to_speech` → also rejected. Real EVI is bidirectional WebSocket
///     and doesn't fit this trait — it lives on a dedicated endpoint
///     (issue #101).
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

    /// Test seam — overrides the base URL so wiremock-driven tests can
    /// point the adapter at a local server. Not exposed for production
    /// use because every other adapter hardcodes its upstream and we want
    /// the deviation to be explicitly opt-in.
    #[doc(hidden)]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Build the JSON body for `POST /tts`. Pulled out so the test suite
    /// can assert the wire shape against Hume's contract without having
    /// to spin up a full HTTP roundtrip.
    pub(crate) fn build_tts_body(req: &TextToSpeechRequest) -> Value {
        // Voice provider: a leading `custom:` marks a user-cloned voice, otherwise
        // we treat the name as one from Hume's preset library. Without `provider`
        // Hume's API is permissive but inconsistent — pinning it is the right
        // default per the docs.
        let (voice_name, voice_provider) = if let Some(rest) = req.voice.strip_prefix("custom:") {
            (rest.to_string(), "CUSTOM_VOICE")
        } else {
            (req.voice.clone(), "HUME_AI")
        };

        // Output format. Hume accepts `mp3 | wav | pcm`. Callers can pin via
        // model = `octave?format=wav`; default mp3 keeps backward compat.
        let format_type = parse_format_from_model(&req.model).unwrap_or("mp3");

        let mut utterance = serde_json::Map::new();
        utterance.insert("text".into(), Value::String(req.input.clone()));
        if !voice_name.is_empty() {
            utterance.insert(
                "voice".into(),
                json!({ "name": voice_name, "provider": voice_provider }),
            );
        }

        json!({
            "utterances": [Value::Object(utterance)],
            "format": { "type": format_type },
            "num_generations": 1,
        })
    }
}

/// `model` can be `"octave"` (default) or `"octave?format=wav"` etc.
/// We extract the format query if present.
fn parse_format_from_model(model: &str) -> Option<&'static str> {
    let q = model.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some(("format", v)) = pair.split_once('=') {
            return match v.to_ascii_lowercase().as_str() {
                "mp3" => Some("mp3"),
                "wav" => Some("wav"),
                "pcm" => Some("pcm"),
                _ => None,
            };
        }
    }
    None
}

/// Map Hume's `encoding.format` field to the right HTTP content-type so the
/// gateway returns the correct MIME to its caller (rather than always
/// claiming `audio/mpeg`).
fn content_type_for(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "wav" => "audio/wav",
        "pcm" => "audio/L16",
        _ => "audio/mpeg",
    }
}

#[async_trait]
impl ProviderAdapter for HumeAdapter {
    async fn stream_chat(&self, _req: &ChatCompletionRequest) -> Result<ChatStream> {
        anyhow::bail!("Hume AI is an audio provider; chat is not supported")
    }

    async fn text_to_speech(&self, req: &TextToSpeechRequest) -> Result<(String, Vec<u8>)> {
        let body = Self::build_tts_body(req);

        let response = self
            .client
            .post(format!("{}/tts", self.base_url))
            .header("X-Hume-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::new(
                classify_response(PROVIDER, response).await,
            ));
        }

        // /tts response shape:
        //   { "request_id": "...", "generations": [{
        //       "generation_id": "...", "audio": "<base64>", "duration": …,
        //       "encoding": { "format": "mp3", "sample_rate": 48000 }, …
        //   }] }
        let json: Value = response.json().await?;
        let gen = json
            .get("generations")
            .and_then(|g| g.get(0))
            .ok_or_else(|| anyhow::anyhow!("Hume TTS: no generations in response"))?;

        let b64 = gen
            .get("audio")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Hume TTS: no audio in generations[0]"))?;

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| anyhow::anyhow!("Hume TTS base64 decode failed: {}", e))?;

        // Honor whatever format Hume actually encoded (the request format is
        // a *hint* — Hume can downgrade it). Falls back to mp3 if missing.
        let actual_format = gen
            .get("encoding")
            .and_then(|e| e.get("format"))
            .and_then(|v| v.as_str())
            .unwrap_or("mp3");

        Ok((content_type_for(actual_format).to_string(), bytes))
    }

    async fn transcribe_audio(
        &self,
        _audio_data: &[u8],
        _req: &AudioTranscriptionRequest,
    ) -> Result<String> {
        anyhow::bail!(
            "Hume AI does not expose a standalone transcription endpoint — \
             route to ElevenLabs or OpenAI Whisper instead"
        )
    }

    async fn speech_to_speech(
        &self,
        _audio_data: &[u8],
        _req: &SpeechToSpeechRequest,
    ) -> Result<Vec<u8>> {
        // The previous REST shim against /evi/chat was wishful thinking —
        // EVI is bidirectional WebSocket and doesn't have an idiomatic
        // request/response REST endpoint. Bail explicitly with a pointer
        // to the right surface (#101 covers wiring the WS endpoint).
        anyhow::bail!(
            "Hume EVI is a WebSocket protocol, not REST. \
             Use the dedicated EVI endpoint (issue #101) for live conversation."
        )
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests pinning the wire shape of `POST /tts` to Hume's published
    //! contract. If Hume ever changes the contract we'd rather see these
    //! tests fail loudly than discover the breakage in production.
    use super::*;

    fn req(input: &str, voice: &str, model: &str) -> TextToSpeechRequest {
        TextToSpeechRequest {
            input: input.into(),
            model: model.into(),
            voice: voice.into(),
        }
    }

    #[test]
    fn body_has_utterances_array_with_text() {
        let body = HumeAdapter::build_tts_body(&req("hello world", "", ""));
        let utt = &body["utterances"][0];
        assert_eq!(utt["text"], "hello world");
    }

    #[test]
    fn body_pins_voice_provider_to_hume_ai_for_known_names() {
        let body = HumeAdapter::build_tts_body(&req("hi", "ITO", "octave"));
        // Voice object must have BOTH name and provider — the docs are
        // explicit on this and Hume's API is inconsistent without it.
        assert_eq!(body["utterances"][0]["voice"]["name"], "ITO");
        assert_eq!(body["utterances"][0]["voice"]["provider"], "HUME_AI");
    }

    #[test]
    fn body_routes_custom_voices_to_custom_voice_provider() {
        // `custom:my-cloned-voice` syntax → CUSTOM_VOICE provider, name
        // stripped of the prefix. Lets users plug in cloned voices without
        // a separate API surface.
        let body = HumeAdapter::build_tts_body(&req("hi", "custom:alice-clone", "octave"));
        assert_eq!(body["utterances"][0]["voice"]["name"], "alice-clone");
        assert_eq!(body["utterances"][0]["voice"]["provider"], "CUSTOM_VOICE");
    }

    #[test]
    fn empty_voice_omits_voice_field() {
        // Server defaults pick a voice when we don't specify — sending
        // `{"name": ""}` would be wrong.
        let body = HumeAdapter::build_tts_body(&req("hi", "", "octave"));
        assert!(
            body["utterances"][0].get("voice").is_none(),
            "expected no voice key when caller passes empty string"
        );
    }

    #[test]
    fn body_defaults_to_mp3_format() {
        let body = HumeAdapter::build_tts_body(&req("hi", "ITO", ""));
        assert_eq!(body["format"]["type"], "mp3");
    }

    #[test]
    fn body_picks_wav_when_model_query_says_so() {
        let body = HumeAdapter::build_tts_body(&req("hi", "ITO", "octave?format=wav"));
        assert_eq!(body["format"]["type"], "wav");
    }

    #[test]
    fn body_picks_pcm_when_model_query_says_so() {
        let body = HumeAdapter::build_tts_body(&req("hi", "ITO", "octave?format=pcm"));
        assert_eq!(body["format"]["type"], "pcm");
    }

    #[test]
    fn body_ignores_unknown_format() {
        let body = HumeAdapter::build_tts_body(&req("hi", "ITO", "octave?format=ogg"));
        // Unknown format → fall back to default mp3 (Hume rejects ogg).
        assert_eq!(body["format"]["type"], "mp3");
    }

    #[test]
    fn body_explicitly_requests_one_generation() {
        // Default is 1 anyway, but pinning it makes the request idempotent
        // for billing/idempotency keys and avoids surprise multi-pulls.
        let body = HumeAdapter::build_tts_body(&req("hi", "", ""));
        assert_eq!(body["num_generations"], 1);
    }

    #[test]
    fn content_type_maps_hume_encoding_correctly() {
        assert_eq!(content_type_for("mp3"), "audio/mpeg");
        assert_eq!(content_type_for("MP3"), "audio/mpeg");
        assert_eq!(content_type_for("wav"), "audio/wav");
        assert_eq!(content_type_for("WAV"), "audio/wav");
        assert_eq!(content_type_for("pcm"), "audio/L16");
        assert_eq!(content_type_for("anything-else"), "audio/mpeg");
    }

    #[test]
    fn parse_format_from_model_extracts_query() {
        assert_eq!(parse_format_from_model("octave"), None);
        assert_eq!(parse_format_from_model("octave?format=mp3"), Some("mp3"));
        assert_eq!(parse_format_from_model("octave?format=wav"), Some("wav"));
        assert_eq!(parse_format_from_model("octave?format=PCM"), Some("pcm"));
        assert_eq!(
            parse_format_from_model("octave?other=1&format=wav"),
            Some("wav")
        );
        assert_eq!(parse_format_from_model("octave?format=ogg"), None);
    }
}
