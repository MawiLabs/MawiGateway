use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request (OpenAI format)
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub response_format: Option<ResponseFormat>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub modality: Option<String>,  // "text" | "multimodal" | "image" | etc.
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub type_: String, // "text" or "json_object"
}

/// Chat completion response (OpenAI format)
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

/// Streaming chunk (OpenAI SSE format)
#[derive(Debug, Serialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Serialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Image generation request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ImageGenerationRequest {
    pub prompt: String,
    pub model: String,
    #[serde(default = "default_n")]
    pub n: u32,
    #[serde(default = "default_size")]
    pub size: String,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
}

fn default_n() -> u32 { 1 }
fn default_size() -> String { "1024x1024".to_string() }

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageGenerationResponse {
    pub created: u64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    pub revised_prompt: Option<String>,
}

/// Text-to-speech request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct TextToSpeechRequest {
    pub input: String,
    pub model: String,
    pub voice: String,
}

/// Speech-to-text (transcription) request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct AudioTranscriptionRequest {
    pub model: String,
    #[serde(default)]
    pub language: Option<String>,
}

/// Speech-to-text response
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct AudioTranscriptionResponse {
    pub text: String,
}

/// Speech-to-speech request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct SpeechToSpeechRequest {
    pub model: String,
    #[serde(default)]
    pub voice: Option<String>,
}

/// Video generation request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct VideoGenerationRequest {
    pub prompt: String,
    pub model: String,
    #[serde(default)]
    pub size: Option<String>,  // e.g., "1280x720", "1920x1080"
    #[serde(default)]
    pub duration: Option<u32>,  // Duration in seconds
}

/// Video generation response
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct VideoGenerationResponse {
    pub url: Option<String>,     // URL to video
    pub data: Option<String>,     // Base64 encoded video data
    pub format: String,           // e.g., "mp4", "webm"
}

// ============================================================================
// OpenAI /responses Endpoint Types (GPT-5 Multimodal)
// ============================================================================

/// Request for OpenAI /responses endpoint (unified multimodal API)
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ResponsesRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
}

/// Single output item from /responses (can be text, image, or tool call)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ResponsesOutput {
    #[serde(rename = "type")]
    pub output_type: String, // "text" | "image" | "tool_call"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ResponsesImageOutput>,
}

/// Image output from /responses endpoint
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ResponsesImageOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Response from OpenAI /responses endpoint
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ResponsesResponse {
    pub id: String,
    pub output: Vec<ResponsesOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponsesUsage>,
}

/// Token usage for /responses endpoint
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ResponsesUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}
