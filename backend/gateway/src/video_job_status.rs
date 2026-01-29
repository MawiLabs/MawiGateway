use poem::{web::Data, handler};
use poem_openapi::param::Path;
use crate::executor::Executor;

#[handler]
pub async fn get_video_job_status(
    Path(job_id): Path<String>,
    executor: Data<&Executor>,
) -> poem::Result<poem::web::Json<serde_json::Value>> {
    eprintln!("📊 Checking video job status: {}", job_id);
    
    // Poll Azure for job status
    let status = executor.poll_video_job(&job_id).await
        .map_err(|e| poem::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;
    
    Ok(poem::web::Json(status))
}
