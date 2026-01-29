use poem::{web::{Data, Multipart}, handler};
use std::sync::Arc;
use crate::executor::Executor;
use mawi_core::types::{AudioTranscriptionRequest, AudioTranscriptionResponse};
use poem::web::Json;

#[handler]
pub async fn transcribe_audio(
    req: &poem::Request,
    mut multipart: Multipart,
    executor: Data<&Arc<Executor>>,
) -> poem::Result<Json<AudioTranscriptionResponse>> {


    // 1. Authenticate Request
    let user = req.extensions().get::<mawi_core::auth::User>()
        .ok_or_else(|| poem::Error::from_string("Authentication required", poem::http::StatusCode::UNAUTHORIZED))?;

    let mut audio_data: Option<Vec<u8>> = None;
    let mut model: Option<String> = None;
    let mut language: Option<String> = None;

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
            Some("language") => {
                let text = field.text().await.ok();
                language = text;
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

    eprintln!("🎤 STT Request for model: {} ({} bytes) by user {}", model, audio_data.len(), user.id);

    let request_obj = AudioTranscriptionRequest {
        model,
        language,
    };

    let text = executor
        .execute_transcription(&audio_data, &request_obj, &user.id)
        .await
        .map_err(|e| poem::Error::from_string(
            format!("Transcription failed: {}", e),
            poem::http::StatusCode::INTERNAL_SERVER_ERROR
        ))?;

    Ok(Json(AudioTranscriptionResponse { text }))
}
