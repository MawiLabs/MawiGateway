use async_trait::async_trait;
use tokio_stream::Stream;
use std::pin::Pin;
use crate::types::{ChatCompletionRequest, ImageGenerationRequest, ImageGenerationResponse, 
    TextToSpeechRequest, AudioTranscriptionRequest, SpeechToSpeechRequest,
    VideoGenerationRequest, VideoGenerationResponse};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<String, anyhow::Error>> + Send>>;

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Stream chat completion from provider
    async fn stream_chat(&self, req: &ChatCompletionRequest) -> Result<ChatStream, anyhow::Error>;
    
    /// Non-streaming chat (default impl uses stream)
    async fn chat(&self, req: &ChatCompletionRequest) -> Result<String, anyhow::Error> {
        eprintln!("Starting chat collection for model: {}", req.model);
        let mut stream = self.stream_chat(req).await?;
        let mut content = String::new();
        let mut chunk_count = 0;
        
        use tokio_stream::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(text) => {
                    chunk_count += 1;
                    eprintln!("Chunk #{}: '{}' ({} bytes)", chunk_count, text, text.len());
                    content.push_str(&text);
                }
                Err(e) => {
                    eprintln!("Stream error: {}", e);
                    return Err(e);
                }
            }
        }
        
        eprintln!("Chat collection complete: {} chunks, {} total bytes", chunk_count, content.len());
        eprintln!("Final content: '{}'", content);
        Ok(content)
    }

    /// Generate images from provider
    async fn generate_image(&self, _req: &ImageGenerationRequest) -> Result<ImageGenerationResponse, anyhow::Error> {
        Err(anyhow::anyhow!("Image generation not supported by this provider"))
    }

    /// Generate speech from text (returns (content_type, bytes))
    async fn text_to_speech(&self, _req: &TextToSpeechRequest) -> Result<(String, Vec<u8>), anyhow::Error> {
        Err(anyhow::anyhow!("Text-to-speech not supported by this provider"))
    }

    /// Transcribe audio to text (audio_data is audio file bytes)
    async fn transcribe_audio(&self, _audio_data: &[u8], _req: &AudioTranscriptionRequest) -> Result<String, anyhow::Error> {
        Err(anyhow::anyhow!("Speech-to-text not supported by this provider"))
    }

    /// Speech-to-speech conversion (returns audio bytes)
    async fn speech_to_speech(&self, _audio_data: &[u8], _req: &SpeechToSpeechRequest) -> Result<Vec<u8>, anyhow::Error> {
        Err(anyhow::anyhow!("Speech-to-speech not supported by this provider"))
    }

    /// Generate video from text prompt (returns (content_type, video_bytes))
    async fn generate_video(&self, _req: &VideoGenerationRequest) -> Result<VideoGenerationResponse, anyhow::Error> {
        Err(anyhow::anyhow!("Video generation not supported by this provider"))
    }

    /// Poll video generation job status and return video URL when ready
    async fn poll_video_job(&self, _job_id: &str) -> Result<serde_json::Value, anyhow::Error> {
        Err(anyhow::anyhow!("Async video generation not supported by this provider"))
    }

    /// Get video content bytes
    async fn get_video_content(&self, _generation_id: &str) -> Result<Vec<u8>, anyhow::Error> {
        Err(anyhow::anyhow!("Video content fetching not supported by this provider"))
    }
}

pub mod openai;
pub mod azure;
pub mod gemini;
pub mod anthropic;
pub mod xai;
pub mod mistral;
pub mod perplexity;
pub mod selfhosted;
pub mod deepseek;
pub mod elevenlabs;

pub use openai::OpenAIAdapter;
pub use azure::AzureProvider;
pub use gemini::GeminiAdapter;
pub use anthropic::AnthropicAdapter;
pub use xai::XaiAdapter;
pub use mistral::MistralAdapter;
pub use perplexity::PerplexityAdapter;
pub use selfhosted::SelfHostedAdapter;
pub use deepseek::DeepSeekAdapter;
pub use elevenlabs::ElevenLabsAdapter;
