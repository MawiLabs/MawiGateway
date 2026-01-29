use poem::{web::{Data, Multipart}, handler, Response, Body};
use std::sync::Arc;
use crate::executor::Executor;
use mawi_core::types::SpeechToSpeechRequest;

#[handler]
pub async fn speech_to_speech_endpoint(
    mut multipart: Multipart,
    executor: Data<&Arc<Executor>>,
) -> poem::Result<Response> {
    let mut audio_data: Option<Vec<u8>> = None;
    let mut model: Option<String> = None;
    let mut voice: Option<String> = None;

    // Parse multipart form data
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().map(|s| s.to_string());
        
        match name.as_deref() {
            Some("file") => {
                let bytes = field.bytes().await
                    .map_err(|e| poem::Error::from_string(
                        format!("Failed to read audio file: {}", e),
                        poem::http::StatusCode::BAD_REQUEST
                    ))?;
                audio_data = Some(bytes.to_vec());
            }
            Some("model") => {
                let text = field.text().await
                    .map_err(|e| poem::Error::from_string(
                        format!("Failed to read model: {}", e),
                        poem::http::StatusCode::BAD_REQUEST
                    ))?;
                model = Some(text);
            }
            Some("voice") => {
                let text = field.text().await.ok();
                voice = text;
            }
            _ => {}
        }
    }

    let audio_data = audio_data.ok_or_else(|| 
        poem::Error::from_string("Missing 'file' field", poem::http::StatusCode::BAD_REQUEST)
    )?;
    
    let model = model.ok_or_else(|| 
        poem::Error::from_string("Missing 'model' field", poem::http::StatusCode::BAD_REQUEST)
    )?;

    eprintln!("🔄 STS Request for model: {} ({} bytes)", model, audio_data.len());

    let request = SpeechToSpeechRequest {
        model,
        voice,
    };

    let result_audio = executor
        .execute_speech_to_speech(&audio_data, &request)
        .await
        .map_err(|e| poem::Error::from_string(
            format!("Speech-to-speech failed: {}", e),
            poem::http::StatusCode::INTERNAL_SERVER_ERROR
        ))?;

    Ok(Response::builder()
        .content_type("audio/mpeg")
        .body(Body::from(result_audio)))
}
