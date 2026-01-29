use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::mcp_client::McpManager;
use mawi_core::providers::{ProviderAdapter, AzureProvider, OpenAIAdapter, GeminiAdapter, AnthropicAdapter, XaiAdapter, MistralAdapter, PerplexityAdapter, SelfHostedAdapter, DeepSeekAdapter, ElevenLabsAdapter};
use moka::future::Cache;
use std::time::Duration;
use tracing::{debug, info, warn, error};

use mawi_core::types::{ChatCompletionRequest, ImageGenerationRequest, ImageGenerationResponse};
use mawi_core::unified::{UnifiedChatRequest, UnifiedChatResponse, ChatChoice, ChatMessage, TokenUsage, RoutingMetadata, RequestedRouting, ActualRouting, AgenticStreamEvent};
use anyhow::{Result, Context};
use futures::Stream;

pub struct Executor {
    pub pool: PgPool,
    pub http_client: reqwest::Client,
    model_cache: Cache<String, mawi_core::models::Model>,
    logger: Arc<RequestLogger>,
    provider_cache: Cache<String, mawi_core::models::Provider>,
    service_cache: Cache<String, mawi_core::services::Service>,
    service_models_cache: Cache<String, Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)>>,
    quota_worker: Arc<QuotaWorker>,
    #[allow(dead_code)]
    providers: HashMap<String, Arc<dyn ProviderAdapter>>,
    pub mcp_manager: Arc<RwLock<McpManager>>,
    pub circuit_breaker: Arc<crate::circuit_breaker::CircuitBreaker>,
}

// async quota charging (prevents task explosion)
struct QuotaTask {
    user_id: String,
    cost: f64,
    pool: PgPool,
}

struct QuotaWorker {
    sender: tokio::sync::mpsc::Sender<QuotaTask>,
}

impl QuotaWorker {
    fn new(pool: PgPool, num_workers: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<QuotaTask>(1000);
        let rx = Arc::new(tokio::sync::Mutex::new(rx));
        

        for worker_id in 0..num_workers {
            let mut rx = Arc::clone(&rx);
            tokio::spawn(async move {
                loop {
                    let task = {
                        let mut rx_guard = rx.lock().await;
                        rx_guard.recv().await
                    };
                    
                    match task {
                        Some(task) => {
                            let quota_manager = mawi_core::quota::QuotaManager::new(task.pool);
                            if let Err(e) = quota_manager.charge_user(&task.user_id, task.cost).await {
                                eprintln!("[Worker {}] Failed to charge user {}: {}", worker_id, task.user_id, e);
                            }
                        }
                        None => break,
                    }
                }
            });
        }
        
        Self { sender: tx }
    }
    
    fn charge(&self, user_id: String, cost: f64, pool: PgPool) {
        let _ = self.sender.try_send(QuotaTask { user_id, cost, pool });
    }
}

pub struct RequestLogger {
    sender: tokio::sync::mpsc::Sender<LogEntry>,
}

struct LogEntry {
    pool: PgPool,
    params: LogParams,
}

pub struct LogParams {
    pub id: String,
    pub key_id: Option<String>,
    pub service: String,
    pub model_id: String,
    pub provider_type: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub latency_ms: i64,
    pub latency_us: i64,
    pub status: String,
    pub error: Option<String>,
    pub failover_count: i32,
    pub cost_usd: Option<f64>,
    pub user_id: Option<String>,
}

impl RequestLogger {
    pub fn new(pool: PgPool) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<LogEntry>(10000);
        
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(500);
            let mut last_flush = std::time::Instant::now();
            
            loop {
                tokio::select! {
                    Some(entry) = rx.recv() => {
                        batch.push(entry);
                        
                        // Flush when batch reaches 500 or 100ms timeout
                        if batch.len() >= 500 || last_flush.elapsed() > std::time::Duration::from_millis(100) {
                            Self::flush_batch(&mut batch).await;
                            last_flush = std::time::Instant::now();
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        if !batch.is_empty() {
                            Self::flush_batch(&mut batch).await;
                            last_flush = std::time::Instant::now();
                        }
                    }
                }
            }
        });
        
        Self { sender: tx }
    }
    
    async fn flush_batch(batch: &mut Vec<LogEntry>) {
        if batch.is_empty() { return; }
        
        // Defensive: Get pool safely
        let pool = match batch.first() {
            Some(entry) => entry.pool.clone(),
            None => return, // Race condition safety
        };
        
        // Build multi-row insert query
        let placeholders: Vec<String> = (0..batch.len())
            .map(|i| {
                let base = i * 15 + 1;
                format!("(${},${},${},${},${},${},${},${},${},${},${},${},${},${},${})",
                    base, base+1, base+2, base+3, base+4, base+5, base+6, base+7,
                    base+8, base+9, base+10, base+11, base+12, base+13, base+14)
            })
            .collect();
        
        let query = format!(
            "INSERT INTO request_logs (id, virtual_key_id, service_name, model_id, provider_type, \
             tokens_prompt, tokens_completion, tokens_total, latency_ms, latency_us, status, \
             error_message, failover_count, cost_usd, user_id) VALUES {}",
            placeholders.join(",")
        );
        
        let mut q = sqlx::query(&query);
        
        // Bind each entry's values
        for entry in batch.iter() {
            q = q.bind(&entry.params.id)
                .bind(&entry.params.key_id)
                .bind(&entry.params.service)
                .bind(&entry.params.model_id)
                .bind(&entry.params.provider_type)
                .bind(&entry.params.prompt_tokens)
                .bind(&entry.params.completion_tokens)
                .bind(&entry.params.total_tokens)
                .bind(&entry.params.latency_ms)
                .bind(&entry.params.latency_us)
                .bind(&entry.params.status)
                .bind(&entry.params.error)
                .bind(&entry.params.failover_count)
                .bind(&entry.params.cost_usd)
                .bind(&entry.params.user_id);
        }
        
        let _ = q.execute(&pool).await;
        batch.clear();
    }
    
    pub fn log(&self, pool: PgPool, params: LogParams) {
        if let Err(_) = self.sender.try_send(LogEntry { pool, params }) {
            crate::metrics::LOG_DROPS.inc();
        } else {
            // Approximate buffer depth (channel capacity - available space)
            crate::metrics::LOG_BUFFER_DEPTH.set(
                (10000 - self.sender.capacity()) as i64
            );
        }
    }
}

impl Executor {
    pub fn new(pool: PgPool, mcp_manager: Arc<RwLock<McpManager>>) -> Self {
        let providers: HashMap<String, Arc<dyn ProviderAdapter>> = HashMap::new();
        
        // Configure high-performance connection pooling
        let http_client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .timeout(std::time::Duration::from_secs(120))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client - check TLS/network configuration");

        let pool_for_logger = pool.clone();
        let pool_for_quota = pool.clone();

        Self { 
            pool, 
            http_client, 
            providers,
            model_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            provider_cache: Cache::builder()
                .max_capacity(1_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            service_cache: Cache::builder()
                .max_capacity(1_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            service_models_cache: Cache::builder()
                .max_capacity(5_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            quota_worker: Arc::new(QuotaWorker::new(pool_for_quota, 10)),
            logger: Arc::new(RequestLogger::new(pool_for_logger)),
            mcp_manager,
            circuit_breaker: Arc::new(crate::circuit_breaker::CircuitBreaker::new()),
        }
    }
    
    pub fn with_client(pool: PgPool, http_client: reqwest::Client, mcp_manager: Arc<RwLock<McpManager>>) -> Self {
         let pool_for_logger = pool.clone();
         let pool_for_quota = pool.clone();
         Self { 
            pool, 
            http_client, 
            providers: HashMap::new(),
            model_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            provider_cache: Cache::builder()
                .max_capacity(1_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            service_cache: Cache::builder()
                .max_capacity(1_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            service_models_cache: Cache::builder()
                .max_capacity(5_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            quota_worker: Arc::new(QuotaWorker::new(pool_for_quota, 10)),
            logger: Arc::new(RequestLogger::new(pool_for_logger)),
            mcp_manager,
            circuit_breaker: Arc::new(crate::circuit_breaker::CircuitBreaker::new()),
        }
    }

    /// Execute image generation request
    pub async fn execute_image_generation(&self, request: &ImageGenerationRequest, user_id: &str) -> Result<ImageGenerationResponse> {
        let estimated_cost = crate::pricing::PRICING.get_image_cost(&request.model, request.n.max(1) as i64);
        let quota_manager = mawi_core::quota::QuotaManager::new(self.pool.clone());
        quota_manager.check_quota(user_id, estimated_cost).await?;

        let model = self.get_model(&request.model).await?;
        let provider = self.get_provider(&model.provider).await?;
        let adapter = self.create_adapter(&provider, &model)?;
        let response = adapter.generate_image(request).await?;

        // Bill for actual images generated
        let cost = crate::pricing::PRICING.get_image_cost(&request.model, response.data.len() as i64);

        if let Err(e) = quota_manager.charge_user(user_id, cost).await {
            warn!(error = %e, user_id, "failed to charge user for image gen");
        } else {
             debug!(cost, user_id, "charged for image generation");
        }

        Ok(response)
    }

    /// Execute text-to-speech request
    pub async fn execute_text_to_speech(&self, request: &mawi_core::types::TextToSpeechRequest, user_id: &str) -> Result<(String, Vec<u8>)> {
        let estimated_cost = crate::pricing::PRICING.get_tts_cost(&request.model, request.input.len());
        let quota_manager = mawi_core::quota::QuotaManager::new(self.pool.clone());
        quota_manager.check_quota(user_id, 0.01_f64.max(estimated_cost)).await?;

        let model = self.get_model(&request.model).await?;
        let provider = self.get_provider(&model.provider).await?;
        let adapter = self.create_adapter(&provider, &model)?;
        let result = adapter.text_to_speech(request).await?;
        
        if let Err(e) = quota_manager.charge_user(user_id, estimated_cost).await {
             warn!(error = %e, user_id, "failed to charge user for TTS");
        } else {
             debug!(cost = estimated_cost, user_id, "charged for TTS");
        }
        
        Ok(result)
    }

    /// Execute speech-to-text (transcription) request
    pub async fn execute_transcription(&self, audio_data: &[u8], request: &mawi_core::types::AudioTranscriptionRequest, user_id: &str) -> Result<String> {
        let estimated_cost = crate::pricing::PRICING.get_transcription_cost(&request.model);
        
        let quota_manager = mawi_core::quota::QuotaManager::new(self.pool.clone());
        quota_manager.check_quota(user_id, estimated_cost).await?;

        // Resolve model to provider
        let model = self.get_model(&request.model).await?;
        let provider = self.get_provider(&model.provider).await?;
        
        let adapter = self.create_adapter(&provider, &model)?;
        
        let result = adapter.transcribe_audio(audio_data, request).await?;

        if let Err(e) = quota_manager.charge_user(user_id, estimated_cost).await {
             warn!(error = %e, user_id, "failed to charge user for transcription");
        } else {
             debug!(cost = estimated_cost, user_id, "charged for transcription");
        }

        Ok(result)
    }

    /// Execute speech-to-speech request
    pub async fn execute_speech_to_speech(&self, audio_data: &[u8], request: &mawi_core::types::SpeechToSpeechRequest) -> Result<Vec<u8>> {
        // Resolve model to provider
        let model = self.get_model(&request.model).await?;
        let provider = self.get_provider(&model.provider).await?;
        
        let adapter = self.create_adapter(&provider, &model)?;
        
        adapter.speech_to_speech(audio_data, request).await
    }

    /// Execute video generation request
    pub async fn execute_video_generation(&self, request: &mawi_core::types::VideoGenerationRequest, user_id: &str) -> Result<mawi_core::types::VideoGenerationResponse> {
        let estimated_cost = crate::pricing::PRICING.get_video_cost(&request.model);
        let quota_manager = mawi_core::quota::QuotaManager::new(self.pool.clone());
        quota_manager.check_quota(user_id, estimated_cost).await?;

        let model = self.get_model(&request.model).await?;
        let provider = self.get_provider(&model.provider).await?;
        let adapter = self.create_adapter(&provider, &model)?;
        let response = adapter.generate_video(request).await?;
        
        if let Err(e) = quota_manager.charge_user(user_id, estimated_cost).await {
             warn!(error = %e, user_id, "failed to charge user for video");
        } else {
             debug!(cost = estimated_cost, user_id, "charged for video generation");
        }
        
        Ok(response)
    }

    pub async fn poll_video_job(&self, job_id: &str, model_id: &str) -> Result<serde_json::Value> {
        let model = self.get_model(model_id).await?;
        let provider = self.get_provider(&model.provider).await?;
        let adapter = self.create_adapter(&provider, &model)?;
        adapter.poll_video_job(job_id).await
    }

    pub async fn get_video_content(&self, generation_id: &str, model_id: &str) -> Result<Vec<u8>> {
        let model = self.get_model(model_id).await?;
        let provider = self.get_provider(&model.provider).await?;
        let adapter = self.create_adapter(&provider, &model)?;
        adapter.get_video_content(generation_id).await
    }

    /// Execute with streaming support (Agentic only for now)
    pub fn execute_chat_stream(
        &self,
        request: UnifiedChatRequest,
        user_id: &str,
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<AgenticStreamEvent>> + Send>> {
        let pool = self.pool.clone();
        let mcp_manager = self.mcp_manager.clone();
        let http_client = self.http_client.clone();
        let user_id = user_id.to_string(); // Capture for async block
        
        Box::pin(async_stream::try_stream! {
             // Query service type manually to avoid capturing self
             let service_type: Option<String> = sqlx::query_scalar("SELECT service_type FROM services WHERE name = $1")
                 .bind(&request.service)
                 .fetch_optional(&pool)
                 .await
                 .map_err(|e| anyhow::anyhow!("DB Error: {}", e))?;

             if let Some(st) = service_type {
                 if st == "AGENTIC" {
                      let agentic = crate::agentic_executor::AgenticExecutor::new(pool.clone(), mcp_manager.clone());
                      let stream = agentic.execute_stream(request, user_id);
                      for await event in stream {
                          yield event?;
                      }
                      return;
                 }
             }
             
             // Fallback: Create ephemeral Executor for standard execution
             // Reuse existing client to prevent connection churn
             let executor = Executor::with_client(pool, http_client, mcp_manager.clone());
             let response = executor.execute_chat(&request, &user_id).await?;
             if let Some(choice) = response.choices.first() {
                 yield AgenticStreamEvent::FinalResponse(choice.message.content.clone());
             }
        })
    }

    /// Execute request with weighted distribution and automatic failover
    pub async fn execute_chat(
        &self,
        request: &UnifiedChatRequest,
        user_id: &str,
    ) -> Result<UnifiedChatResponse> {
        crate::metrics::HTTP_REQUESTS_TOTAL.inc();
        crate::metrics::REQUESTS_IN_FLIGHT.inc();
        let _timer = crate::metrics::REQUEST_DURATION.start_timer();
        
        // Ensure we decrement in-flight count on function exit
        let _guard = scopeguard::guard((), |_| {
            crate::metrics::REQUESTS_IN_FLIGHT.dec();
        });
        let heuristic_input_tokens = request.messages.iter().map(|m| m.content.len() as i64 / 4).sum::<i64>().max(50);
        let heuristic_output_tokens = request.params.as_ref().and_then(|p| p.max_tokens).unwrap_or(500) as i64;
        
        let (service, models_with_weights) = match self.get_service(&request.service).await {
            Ok(s) => {
                // Check if this is an agentic service - route to agentic executor
                if matches!(s.service_type, mawi_core::services::ServiceType::Agentic) {
                    info!(service = %request.service, "routing to agentic executor");
                    
                    // STRICT QUOTA CHECK for Agentic
                    // Agentic runs are expensive. We check general availability first.
                    let quota_manager = mawi_core::quota::QuotaManager::new(self.pool.clone());
                    let has_quota = quota_manager.check_quota(user_id, 0.05).await?; // Require at least $0.05 for agent start
                    if !has_quota {
                         anyhow::bail!("Insufficient quota for Agentic execution (requires > $0.05)");
                    }
                    
                    let agentic_executor = crate::agentic_executor::AgenticExecutor::new(self.pool.clone(), self.mcp_manager.clone());
                    return agentic_executor.execute(request, user_id).await;
                }
                
                let m = self.get_service_models_with_weights(&request.service).await?;
                (s, m)
            }
            Err(_) => {
                // Fallback: Check if it is a direct model ID
                debug!(service = %request.service, "service not found, checking if model ID");
                let model = self.get_model(&request.service).await.map_err(|_| {
                    anyhow::anyhow!("'{}' is neither a valid Service nor a valid Model", request.service)
                })?;
                
                debug!(model = %model.name, "resolved as direct model");

                // Smart Fallback: Try to find an existing RTCROS config for this model from any service
                // lets users test model with service config applied
                let rtcros_result: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
                    "SELECT rtcros_role, rtcros_task, rtcros_context, rtcros_reasoning, rtcros_output, rtcros_stop
                     FROM service_models 
                     WHERE model_id = $1 
                     AND (rtcros_role IS NOT NULL OR rtcros_task IS NOT NULL)
                     LIMIT 1"
                )
                .bind(&model.id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();
                
                let rtcros_config = rtcros_result
                .map(|(role, task, context, reasoning, output, stop)| {
                     debug!(model = %model.name, "applying existing RTCROS config to direct execution");
                     mawi_core::rtcros::RtcrosConfig {
                        role, task, context, reasoning, output, stop
                     }
                })
                .unwrap_or_else(mawi_core::rtcros::RtcrosConfig::default);
                
                 // Create a virtual service for this single model
                let s = mawi_core::services::Service {
                    name: "direct-execution".to_string(),
                    service_type: mawi_core::services::ServiceType::Pool,
                    description: Some("Direct model execution".to_string()),
                    strategy: "leader-worker".to_string(),
                    guardrails: None,
                    created_at: Some(chrono::Utc::now().timestamp()),
                    pool_type: Some(mawi_core::services::PoolType::SingleModality),
                    input_modalities: vec![mawi_core::services::Modality::Text],
                    output_modalities: vec![mawi_core::services::Modality::Text],
                    planner_model_id: None,
                    system_prompt: None,
                    max_iterations: None,
                    user_id: None,
                };
                
                // Create a single model entry with max weight
                let m = vec![(model.id, model.provider, 100, rtcros_config)];
                (s, m)
            }
        };

        let mut models: Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)> = models_with_weights;
        
        // Check for Model Override (e.g., from Playground scoped testing)
        if let Some(override_model_id) = &request.model {
            debug!(service = %request.service, model = %override_model_id, "model override requested");
            models.retain(|(mid, _, _, _)| mid == override_model_id);
            
            if models.is_empty() {
                // forced model must be in service config
                anyhow::bail!("Model '{}' is not configured for service '{}'", override_model_id, request.service);
            }
        }
        
        // Get ALL models (including unhealthy) to check leader status
        let all_models = self.get_all_service_models(&request.service).await?;

        if models.is_empty() {
            let error_msg = if all_models.is_empty() {
                format!("No models configured for service '{}'", request.service)
            } else {
                // Get detailed health check errors for each model
                let mut model_errors = Vec::new();
                for (model_id, _, _, _) in all_models.iter() {
                    // Get model name
                    let model_name = sqlx::query_as::<_, (String,)>(
                        "SELECT name FROM models WHERE id = $1"
                    )
                    .bind(model_id)
                    .fetch_optional(&self.pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|(name,)| name)
                    .unwrap_or_else(|| model_id.clone());

                    // Get health error
                    let health_error = sqlx::query_as::<_, (Option<String>,)>(
                        "SELECT last_error FROM model_health WHERE model_id = $1"
                    )
                    .bind(model_id)
                    .fetch_optional(&self.pool)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|(err,)| err);

                    if let Some(error) = health_error {
                        model_errors.push(format!("{}: {}", model_name, error));
                    } else {
                        model_errors.push(format!("{}: No health data", model_name));
                    }
                }

                if model_errors.is_empty() {
                    format!("Service '{}' is down - all models unhealthy (no health check data available)", request.service)
                } else {
                    format!(
                        "Service '{}' is down - all models unhealthy: {}",
                        request.service,
                        model_errors.join(", ")
                    )
                }
            };

            // Log the service-level failure
            let error_response = UnifiedChatResponse {
                id: uuid::Uuid::new_v4().to_string(),
                object: "chat.completion".to_string(),
                created: chrono::Utc::now().timestamp(),
                model: all_models.first().map(|(m, _, _, _)| m.clone()).unwrap_or_else(|| "unknown".to_string()),
                choices: vec![],
                usage: None,
                routing_metadata: None,
            };

            self.log_request(
                None,
                &request.service,
                &all_models.first().map(|(m, _, _, _)| m.as_str()).unwrap_or("unknown"),
                &all_models.first().map(|(_, p, _, _)| p.as_str()).unwrap_or("unknown"),
                &error_response,
                0,
                "error",
                Some(&error_msg),
                std::time::Instant::now(), // No real timing for service-level errors
                Some(user_id),
            ).await;

            anyhow::bail!("{}", error_msg);
        }

        let strategy_str = service.strategy.as_str().to_lowercase();
        let selected_models = match strategy_str.as_str() {
            "health" | "leader-worker" | "priority" | "highest_quality" => {
                if matches!(service.service_type, mawi_core::services::ServiceType::Pool) && models.len() > 1 {
                    debug!(strategy = %strategy_str, "using weighted selection for pool service");
                    self.select_weighted(&models)
                } else {
                    debug!(strategy = %strategy_str, "using priority failover strategy");
                    models.to_vec()  // Ordered by position
                }
            },
            "weighted_random" | "weighted" | "random" | "pool" => {
                debug!("using weighted random strategy");
                self.select_weighted(&models)
            },
            "least_cost" => {
                debug!("using least cost strategy");
                self.select_least_cost(&models).await
            },
            "least_latency" | "speed" => {
                debug!("using least latency strategy");
                self.select_least_latency(&models).await
            },
            _ => {
                if matches!(service.service_type, mawi_core::services::ServiceType::Pool) {
                    debug!(from = %strategy_str, "defaulting pool service to weighted strategy");
                    self.select_weighted(&models)
                } else {
                    debug!(strategy = %strategy_str, "unknown strategy, using health");
                    models.to_vec()
                }
            }
        };

        debug!(count = selected_models.len(), service = %request.service, strategy = %service.strategy, "models selected");

        // Execute with failover
        let start_time = std::time::Instant::now();
        let mut last_error = None;
        let mut failover_count = 0;

        for (model_id, provider_id, weight, rtcros_config) in selected_models.iter() {
            debug!(model = %model_id, provider = %provider_id, weight, attempt = failover_count + 1, "attempting model");

            // Circuit Breaker Check
            if !self.circuit_breaker.allow_request(model_id).await {
                warn!(model = %model_id, "circuit breaker open, skipping model");
                last_error = Some(anyhow::anyhow!("Circuit Breaker Open"));
                failover_count += 1; // Count as failure so we try next model
                continue;
            }
            
            let attempt_start = std::time::Instant::now();
            match self.execute_model(model_id, provider_id, request, Some(rtcros_config), user_id).await {
                Ok(response) => {
                    let latency = attempt_start.elapsed().as_millis() as i64;
                    // Passive Health Check: Success
                    self.update_model_health(model_id, true, latency, None).await;
                    // Circuit Breaker: Success
                    self.circuit_breaker.record_success(model_id).await;

                    if failover_count > 0 {
                        info!(model = %model_id, failures = failover_count, "failover successful");
                    } else {
                        debug!(model = %model_id, weight, "primary model succeeded");
                    }
                    
                    // Log success with actual latency
                    self.log_request(
                        None,
                        &request.service,
                        model_id,
                        provider_id,
                        &response,
                        failover_count,
                        "success",
                        None,
                        start_time,
                        Some(user_id),
                    ).await;

                    return Ok(response);
                }
                Err(e) => {
                    let latency = attempt_start.elapsed().as_millis() as i64;
                    // Passive Health Check: Failure
                    self.update_model_health(model_id, false, latency, Some(e.to_string())).await;
                    // Circuit Breaker: Failure
                    self.circuit_breaker.record_failure(model_id).await;

                    crate::metrics::FAILOVER_COUNT.inc();
                    eprintln!("❌ Model {} failed: {}", model_id, e);
                    last_error = Some(e);
                    failover_count += 1;
                    
                    // Log failed request
                    let error_response = UnifiedChatResponse {
                        id: uuid::Uuid::new_v4().to_string(),
                        object: "chat.completion".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        model: model_id.to_string(),
                        choices: vec![],
                        usage: None,
                        routing_metadata: None,
                    };
                    
                    self.log_request(
                        None,
                        &request.service,
                        model_id,
                        provider_id,
                        &error_response,
                        failover_count,
                        "error",
                        last_error.as_ref().map(|e| e.to_string()).as_deref(),
                        start_time,
                        Some(user_id),
                    ).await;
                    
                    // Continue to next model
                    continue;
                }
            }
        }

        // All models failed
        crate::metrics::HTTP_REQUESTS_ERRORS.inc();
        eprintln!("💥 All {} models failed for service '{}'", failover_count, request.service);
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All models failed")))
    }

    async fn execute_model(
        &self,
        model_id: &str,
        provider_id: &str,
        request: &UnifiedChatRequest,
        rtcros: Option<&mawi_core::rtcros::RtcrosConfig>,
        user_id: &str,
    ) -> Result<UnifiedChatResponse> {
        // Get provider
        let provider = self.get_provider(provider_id).await?;
        // Get model details
        let model = self.get_model(model_id).await?;

        // Create adapter
        let adapter = self.create_adapter(&provider, &model)?;

        // SMART CONTEXT PRUNING
        // Use model context window or default to 8192 (safe average)
        let context_limit = if model.context_window > 0 { model.context_window as usize } else { 8192 };
        let pruned_original_messages = crate::context_manager::ContextManager::prune_messages(request.messages.clone(), context_limit);
        
        eprintln!("✂️ Context Manager: Prepared {} messages for model '{}' (Window: {})", 
            pruned_original_messages.len(), model_id, context_limit);

        // Build messages and inject RTCROS if present
        let mut messages: Vec<mawi_core::types::ChatMessage> = Vec::new(); // define messages vec
        
        // Add RTCROS system prompt first
        if let Some(config) = rtcros {
            if let Some(system_prompt) = config.build_system_prompt() {
                messages.push(mawi_core::types::ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                });
            }
        }

        messages.extend(pruned_original_messages.iter().map(|m| {
            mawi_core::types::ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }
        }));

        let estimated_input = messages.iter().map(|m| m.content.len() as i64 / 4).sum::<i64>().max(10);
        let estimated_output = 100; 
        let estimated_cost = crate::pricing::PRICING.estimate_cost(&model.name, &provider.provider_type, estimated_input, estimated_output);
        
        eprintln!("📊 Estimated cost for {}: ${:.6}", model.name, estimated_cost);

        // STRICT CHECK:
        let quota_manager = mawi_core::quota::QuotaManager::new(self.pool.clone());
        let has_enough = quota_manager.check_quota(user_id, 0.01_f64.max(estimated_cost)).await.unwrap_or(false);
        
        if !has_enough {
             eprintln!("🛑 Blocked user {} from running {} (Insufficient quota)", user_id, model.name);
             return Err(anyhow::anyhow!("Insufficient quota. Estimated: ${:.6}", estimated_cost));
        }

        let chat_request = ChatCompletionRequest {
            model: model.name.clone(),
            messages,
            temperature: request.params.as_ref().and_then(|p| p.temperature.map(|t| t as f32)),
            max_tokens: request.params.as_ref().and_then(|p| p.max_tokens),
            stream: false,
            response_format: request.response_format.clone(),
            reasoning_effort: request.params.as_ref().and_then(|p| p.reasoning_effort.clone()),
            modality: Some(model.modality.clone()),
        };
        
        // Pass user_id (as key_id for logging) 
        // Executor::log_request uses `key_id` arg... we need to make sure we use it right?
        // Wait, log_request takes `Option<&str> key_id`.
        // In the original code we passed `None`. Now we have `user_id`. We should pass it?
        // Yes, pass `Some(user_id)`.
        
        // Call the actual provider API
        
        // Call the actual provider API
        eprintln!("Calling provider {} for model {}", provider.provider_type, model.name);
        
        // Start timer
        let start = std::time::Instant::now();

        let response_text = adapter.chat(&chat_request).await.map_err(|e| {
            eprintln!("Provider call failed: {}", e);
            anyhow::anyhow!("Provider API error: {}", e)
        })?;
        
        let latency = start.elapsed().as_millis() as i32;
        eprintln!("Provider responded in {}ms", latency);

        // Build unified response
        let response = UnifiedChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.name,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: response_text.clone(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            routing_metadata: Some(RoutingMetadata {
                requested_routing: RequestedRouting {
                    service: request.service.clone(),
                    model_override: request.model.clone(),
                    routing_strategy: request.routing_strategy.as_ref().map(|s| format!("{:?}", s)),
                },
                actual_routing: ActualRouting {
                    provider: provider.provider_type.clone(),
                    model: model_id.to_string(),
                    fallback_used: false,
                },
            }),
        };
        
        // Note: Logging is handled by execute_chat method to avoid duplicates
        
        Ok(response)
    }

    /// Execute a model directly by ID (used internally, esp. by agentic executor)
    /// This bypasses service routing to avoid recursion
    pub async fn execute_model_directly(
        &self,
        model_id: &str,
        messages: Vec<ChatMessage>,
        user_id: &str,
        response_format: Option<mawi_core::types::ResponseFormat>,
    ) -> Result<UnifiedChatResponse> {
        // Get model
        let model = self.get_model(model_id).await?;
        let _provider = self.get_provider(&model.provider).await?;

        // Execute directly without service routing
        let request = UnifiedChatRequest {
            service: model_id.to_string(),
            messages,
            model: Some(model_id.to_string()),
            params: None,
            stream: None,
            routing_strategy: None,
            response_format,
        };

        self.execute_model(model_id, &model.provider, &request, None, user_id).await
        // Actually, this method `execute_model_directly` is used by WHO?
        // If it's used by `agentic_executor`... agentic needs it.
        // Let's update signature later if it breaks. For now passing "system" or "bypass" might be dangerous.
        // SAFETY: Only used internally?
        // Let's break it so compiler tells us where it's used.
    }

    /// Execute a model directly in streaming mode (used by agentic executor for thoughts)
    pub async fn execute_model_stream_directly(
        &self,
        model_id: &str,
        messages: Vec<ChatMessage>,
        user_id: &str,
        response_format: Option<mawi_core::types::ResponseFormat>,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<AgenticStreamEvent>> + Send>>> {
        let model = self.get_model(model_id).await?;
        let provider = self.get_provider(&model.provider).await?;
        let adapter = self.create_adapter(&provider, &model)?;

        let request = ChatCompletionRequest {
            model: model.name,
            messages: messages.into_iter().map(|m| mawi_core::types::ChatMessage {
                role: m.role,
                content: m.content
            }).collect(),
            temperature: None,
            max_tokens: None,
            stream: true,
            response_format,
            reasoning_effort: None, // Streaming direct execution (Agentic) usually doesn't need this override yet, or we assume None
            modality: Some(model.modality.clone()),
        };

        // Convert the Provider's byte stream into AgenticStreamEvents
        let stream = adapter.stream_chat(&request).await?;
        
        Ok(Box::pin(async_stream::try_stream! {
            for await chunk_res in stream {
                let chunk: String = chunk_res?;
                // Forward raw text chunks (we'll wrap them in ReasoningDelta upstream)
                yield AgenticStreamEvent::FinalResponse(chunk);
            }
        }))
    }

    // Helper methods
    async fn get_service(&self, name: &str) -> Result<mawi_core::services::Service> {
        if let Some(service) = self.service_cache.get(name).await {
            crate::metrics::CACHE_HITS.inc();
            return Ok(service);
        }

        crate::metrics::CACHE_MISSES.inc();
        let service = sqlx::query_as::<_, mawi_core::services::Service>(
            "SELECT name, service_type, description, strategy, guardrails, created_at FROM services WHERE name = $1"
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Service not found: {}", e))?;

        self.service_cache.insert(name.to_string(), service.clone()).await;

        Ok(service)
    }
    
    async fn get_service_models_with_weights(&self, service_name: &str) -> Result<Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)>> {
        if let Some(models) = self.service_models_cache.get(service_name).await {
            crate::metrics::CACHE_HITS.inc();
            return Ok(models);
        }

        crate::metrics::CACHE_MISSES.inc();
        let models = sqlx::query_as::<_, (String, i32, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(
            "SELECT sm.model_id, sm.weight, m.provider_id,
                    sm.rtcros_role, sm.rtcros_task, sm.rtcros_context, 
                    sm.rtcros_reasoning, sm.rtcros_output, sm.rtcros_stop
             FROM service_models sm
             JOIN models m ON sm.model_id = m.id
             LEFT JOIN model_health h ON m.id = h.model_id
             WHERE sm.service_name = $1
             AND (h.is_healthy = 1 OR h.is_healthy IS NULL)
             ORDER BY sm.position"
        )
        .bind(service_name)
        .fetch_all(&self.pool)
        .await?;

        if models.is_empty() {
            eprintln!("❌ No healthy models found for service '{}'", service_name);
        } else {
            eprintln!("✅ Found {} healthy model(s) for service '{}'", models.len(), service_name);
        }

        // Convert to result format
        let result: Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)> = models.iter().map(|(model_id, weight, provider_id, 
                               role, task, context, reasoning, output, stop)| {
            let config = mawi_core::rtcros::RtcrosConfig {
                role: role.clone(),
                task: task.clone(),
                context: context.clone(),
                reasoning: reasoning.clone(),
                output: output.clone(),
                stop: stop.clone(),
            };
            (model_id.clone(), provider_id.clone(), *weight as i32, config)
        }).collect();

        // 3. Update cache (auto-eviction handled by moka)
        self.service_models_cache.insert(service_name.to_string(), result.clone()).await;

        Ok(result)
    }

    /// Get ALL service models (including unhealthy) for leader status checks
    async fn get_all_service_models(&self, service_name: &str) -> Result<Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)>> {
        let models = sqlx::query_as::<_, (String, i32, String)>(
            "SELECT sm.model_id, sm.weight, m.provider_id
             FROM service_models sm
             JOIN models m ON sm.model_id = m.id
             WHERE sm.service_name = $1
             ORDER BY sm.position"
        )
        .bind(service_name)
        .fetch_all(&self.pool)
        .await?;

        Ok(models.iter().map(|(model_id, weight, provider_id)| {
            (model_id.clone(), provider_id.clone(), *weight as i32, mawi_core::rtcros::RtcrosConfig::default())
        }).collect())
    }

    /// Check if a specific model is unhealthy
    #[allow(dead_code)]
    async fn is_model_unhealthy(&self, model_id: &str) -> bool {
        sqlx::query_as::<_, (i64,)>(
            "SELECT is_healthy FROM model_health WHERE model_id = $1"
        )
        .bind(model_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(|(is_healthy,)| is_healthy == 0)
        .unwrap_or(false)  // If no health data, assume healthy
    }

    async fn get_provider(&self, id: &str) -> Result<mawi_core::models::Provider> {
        if let Some(provider) = self.provider_cache.get(id).await {
            crate::metrics::CACHE_HITS.inc();
            return Ok(provider);
        }

        crate::metrics::CACHE_MISSES.inc();
        let provider = sqlx::query_as::<_, mawi_core::models::Provider>("SELECT * FROM providers WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Provider not found: {}", e))?;

        // 3. Update cache (auto-eviction handled by moka)
        self.provider_cache.insert(id.to_string(), provider.clone()).await;

        Ok(provider)
    }

    async fn get_model(&self, id: &str) -> Result<mawi_core::models::Model> {
        if let Some(model) = self.model_cache.get(id).await {
            crate::metrics::CACHE_HITS.inc();
            return Ok(model);
        }

        crate::metrics::CACHE_MISSES.inc();
        let model = sqlx::query_as::<_, mawi_core::models::Model>("SELECT * FROM models WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Model not found: {}", e))?;

        // 3. Update cache (auto-eviction handled by moka)
        self.model_cache.insert(id.to_string(), model.clone()).await;

        Ok(model)
    }

    async fn update_model_health(&self, model_id: &str, is_success: bool, latency_ms: i64, error_msg: Option<String>) {
        let timestamp = chrono::Utc::now().timestamp();
        if is_success {
             let _ = sqlx::query(
                "INSERT INTO model_health (model_id, is_healthy, last_check, response_time_ms, consecutive_failures, last_error)
                 VALUES (?, 1, ?, ?, 0, NULL)
                 ON CONFLICT(model_id) DO UPDATE SET
                    is_healthy = 1,
                    last_check = excluded.last_check,
                    response_time_ms = excluded.response_time_ms,
                    consecutive_failures = 0,
                    last_error = NULL"
            )
            .bind(model_id)
            .bind(timestamp)
            .bind(latency_ms)
            .execute(&self.pool)
            .await;
        } else {
            let _ = sqlx::query(
                "INSERT INTO model_health (model_id, is_healthy, last_check, response_time_ms, consecutive_failures, last_error)
                 VALUES (?, 1, ?, ?, 1, ?) 
                 ON CONFLICT(model_id) DO UPDATE SET
                    last_check = excluded.last_check,
                    consecutive_failures = model_health.consecutive_failures + 1,
                    is_healthy = CASE WHEN model_health.consecutive_failures + 1 >= 5 THEN 0 ELSE 1 END,
                    last_error = excluded.last_error"
            )
            .bind(model_id)
            .bind(timestamp)
            .bind(latency_ms)
            .bind(error_msg)
            .execute(&self.pool)
            .await;
        }
    }

    fn select_weighted(&self, models: &[(String, String, i32, mawi_core::rtcros::RtcrosConfig)]) -> Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)> {
        if models.is_empty() {
            return vec![];
        }

        // Calculate total weight
        let total_weight: i32 = models.iter().map(|(_, _, w, _)| w).sum();
        
        // Add tolerance for weight sum (99-101 is acceptable)
        // Check for zero total weight
        if total_weight <= 0 {
            eprintln!("⚠️ Invalid zero total weight, treating as equal distribution");
            return models.to_vec();
        }

        // Auto-normalize: If sum != 100, we just roll against the actual sum.
        // e.g. 70 + 20 = 90. Roll 0..90. 70/90 chance for A.
        if total_weight < 99 || total_weight > 101 {
             eprintln!("ℹ️  Weights sum to {} (not 100), using relative distribution.", total_weight);
        }

        // Generate random number between 0 and total_weight
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut roll = rng.gen_range(0..total_weight);

        eprintln!("Weighted selection: total_weight={}, roll={}", total_weight, roll);

        // Find selected model
        for (i, (model_id, _provider_id, weight, _)) in models.iter().enumerate() {
            eprintln!("  Model {}: weight={}, roll={}", model_id, weight, roll);
            if roll < *weight {
                eprintln!("  → Selected model {} (weight {})", model_id, weight);
                // Return selected model first, then others as fallback
                let mut result = vec![models[i].clone()];
                result.extend(models.iter().enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, m)| m.clone()));
                return result;
            }
            roll -= weight;
        }

        // Fallback (should never happen)
        models.to_vec()
    }

        async fn select_least_cost(&self, models: &[(String, String, i32, mawi_core::rtcros::RtcrosConfig)]) -> Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)> {
        if models.len() <= 1 { return models.to_vec(); }
        
        let mut models_with_cost = Vec::new();
        for (model_id, provider_id, weight, rtcros) in models {
            // Get model name, provider type, and database pricing for cost calculation
            let model_info = sqlx::query_as::<_, (String, Option<f64>, String)>(
                "SELECT m.name, m.cost_per_1k_input_tokens, p.provider_type 
                 FROM models m 
                 JOIN providers p ON m.provider_id = p.id 
                 WHERE m.id = $1"
            )
                .bind(model_id)
                .fetch_one(&self.pool)
                .await
                .ok();

            let (model_name, db_cost, provider_type) = model_info
                .unwrap_or_else(|| (model_id.clone(), None, "unknown".to_string()));

            // Priority 1: Self-hosted/Ollama = FREE ($0)
            let cost = if provider_type.eq_ignore_ascii_case("selfhosted") 
                        || provider_type.eq_ignore_ascii_case("ollama") {
                eprintln!("📍 Model '{}' is self-hosted -> $0 cost", model_name);
                0.0
            }
            // Priority 2: Use database-stored pricing if available
            else if let Some(c) = db_cost {
                eprintln!("📍 Model '{}' has DB cost: ${}/1k tokens", model_name, c);
                c
            }
            // Priority 3: Fall back to static pricing map
            else if let Some(p) = crate::pricing::PRICING.get_pricing(&model_name) {
                p.prompt_price_per_million / 1000.0  // Convert to per 1k
            }
            // Priority 4: Default for unknown paid providers
            else {
                eprintln!("⚠️ No pricing for '{}' (provider: {}), using fallback $5", model_name, provider_type);
                5.0
            };
            
            models_with_cost.push((cost, (model_id.clone(), provider_id.clone(), *weight, rtcros.clone())));
        }

        // Sort by cost ascending with NaN guards
        models_with_cost.sort_by(|a, b| {
            match (a.0.is_nan(), b.0.is_nan()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater, // NaN goes last
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal),
            }
        });
        
        models_with_cost.into_iter().map(|(_, m)| m).collect()
    }

    async fn select_least_latency(&self, models: &[(String, String, i32, mawi_core::rtcros::RtcrosConfig)]) -> Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)> {
        if models.len() <= 1 { return models.to_vec(); }

        let mut models_with_latency = Vec::new();
        for (model_id, provider_id, weight, rtcros) in models {
            // Time-windowed latency (last 1 hour, successful requests only)
            let latency: i64 = sqlx::query_as::<_, (Option<i64>,)>(
                "SELECT AVG(latency_ms) 
                 FROM request_logs 
                 WHERE model_id = $1 
                   AND created_at > EXTRACT(EPOCH FROM NOW())::BIGINT - 3600
                   AND status = 'success'
                 LIMIT 100"
            )
            .bind(model_id)
            .fetch_one(&self.pool)
            .await
            .ok()
            .and_then(|(l,)| l)
            .unwrap_or_else(|| {
                eprintln!("⚠️ No recent latency data for '{}', using default", model_id);
                1000 // Default to 1s if no data
            });

            models_with_latency.push((latency, (model_id.clone(), provider_id.clone(), *weight, rtcros.clone())));
        }

        // Sort by latency ascending
        models_with_latency.sort_by_key(|a| a.0);
        
        models_with_latency.into_iter().map(|(_, m)| m).collect()
    }

    fn select_round_robin(&self, models: &[(String, String, i32, mawi_core::rtcros::RtcrosConfig)]) -> Vec<(String, String, i32, mawi_core::rtcros::RtcrosConfig)> {
        // For now, just return as is (equivalent to Health/Failover if no state is maintained)
        // Proper round-robin would require persistent or atomic state
        models.to_vec()
    }

    pub async fn log_request(
        &self,
        key_id: Option<&str>,
        service: &str,
        model_id: &str,
        provider_id: &str,
        response: &UnifiedChatResponse,
        failover_count: i32,
        status: &str,
        error: Option<&str>,
        start_time: std::time::Instant,
        user_id: Option<&str>,
    ) {
        let provider = self.get_provider(provider_id).await.ok();
        
        // Calculate latency in microseconds
        let latency_us = start_time.elapsed().as_micros() as i64;
        let latency_ms = (latency_us / 1000) as i64;
        
        // Calculate cost based on token usage and model pricing
        let cost_usd = if let Some(usage) = &response.usage {
            crate::pricing::PRICING.calculate_cost(
                &response.model,
                usage.prompt_tokens as i64,
                usage.completion_tokens as i64
            )
        } else {
            None
        };

        // Sanitize error message to hide API keys
        let sanitized_error = error.map(|e| {
            // lazy_static regex for performance
            // Matches OpenAI (sk-...) and Google (AIza...) style keys
            use regex::Regex;
            use std::sync::OnceLock;
            
            static API_KEY_REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            let re = API_KEY_REGEX.get_or_init(|| {
                regex::Regex::new(r"(sk-[a-zA-Z0-9\-_]{20,}|AIza[a-zA-Z0-9\-_]{20,})").expect("Invalid regex")
            });

            re.replace_all(e, |caps: &regex::Captures| {
                let key = &caps[0];
                if key.len() > 7 {
                    format!("{}...", &key[0..7])
                } else {
                    "***".to_string()
                }
            }).to_string()
        });
        
        let pool = self.pool.clone();
        
        // Clone owned data for closure
        let log_id = uuid::Uuid::new_v4().to_string();
        let key_id_owned = key_id.map(|s| s.to_string());
        let service_owned = service.to_string();
        let model_id_owned = model_id.to_string();
        let provider_type_owned = provider.as_ref().map(|p| p.provider_type.clone());
        // REMOVED usage clone as TokenUsage is not Clone and we don't need ownership here
        let status_owned = status.to_string();
        let user_id_owned = user_id.map(|s| s.to_string());
        let cost_usd_owned = cost_usd;

        // Fire and forget via channel
        self.logger.log(pool.clone(), LogParams {
            id: log_id,
            key_id: key_id_owned.clone(), // Clone needed for charging below
            service: service_owned,
            model_id: model_id_owned,
            provider_type: provider_type_owned,
            prompt_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
            completion_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
            total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
            latency_ms,
            latency_us,
            status: status_owned,
            error: sanitized_error,
            failover_count,
            cost_usd: cost_usd_owned,
            user_id: user_id_owned.clone(), // Clone needed for charging below
        });
        
        // Charge user via worker pool (bounded concurrency)
        if let Some(cost) = cost_usd_owned {
            if let Some(uid) = user_id_owned.as_deref().or(key_id_owned.as_deref()) {
                self.quota_worker.charge(uid.to_string(), cost, pool.clone());
            }
        }
    }

    /// Create provider adapter with resolved credentials
    fn create_adapter(&self, provider: &mawi_core::models::Provider, model: &mawi_core::models::Model) -> Result<Arc<dyn ProviderAdapter>> {
        // Resolve credentials (prefer model override)
        let raw_api_key = model.api_key.as_deref()
            .or(provider.api_key.as_deref())
            .unwrap_or("")
            .to_string();

        let api_key = if !raw_api_key.is_empty() {
             mawi_core::security::decrypt_key(&raw_api_key).unwrap_or_else(|e| {
                 eprintln!("⚠️ Failed to decrypt API key for provider {}: {}", provider.name, e);
                 raw_api_key.clone() // Fallback to raw (in case of migration or plain env vars)
             })
        } else {
             String::new()
        };

        let base_url = model.api_endpoint.as_deref()
            .or(provider.api_endpoint.as_deref())
            .unwrap_or("")
            .to_string();

        let api_version = model.api_version.as_deref()
            .or(provider.api_version.as_deref())
            .map(|s| s.to_string());

        match provider.provider_type.to_lowercase().as_str() {
            "openai" => Ok(Arc::new(OpenAIAdapter::new(self.http_client.clone(), api_key))),
            "azure" => {
                if base_url.is_empty() {
                    anyhow::bail!("Azure provider requires api_endpoint (Base URL)");
                }
                Ok(Arc::new(AzureProvider::new(self.http_client.clone(), api_key, base_url, api_version)))
            },
            "google" | "gemini" => Ok(Arc::new(GeminiAdapter::new(self.http_client.clone(), api_key))),
            "anthropic" => Ok(Arc::new(AnthropicAdapter::new(self.http_client.clone(), api_key))),
            "xai" => Ok(Arc::new(XaiAdapter::new(self.http_client.clone(), api_key))),
            "mistral" => Ok(Arc::new(MistralAdapter::new(self.http_client.clone(), api_key))),
            "perplexity" => Ok(Arc::new(PerplexityAdapter::new(self.http_client.clone(), api_key))),
            "deepseek" => Ok(Arc::new(DeepSeekAdapter::new(self.http_client.clone(), api_key))),
            "elevenlabs" => Ok(Arc::new(ElevenLabsAdapter::new(self.http_client.clone(), api_key))),
            "selfhosted" | "ollama" => {
                 // Self-Hosted / Ollama
                if base_url.is_empty() {
                     if provider.provider_type == "ollama" {
                         // Default for ollama
                         Ok(Arc::new(SelfHostedAdapter::new(self.http_client.clone(), api_key, "http://localhost:11434".to_string())))
                     } else {
                        anyhow::bail!("Self-hosted provider requires api_endpoint (Base URL)");
                     }
                } else {
                    Ok(Arc::new(SelfHostedAdapter::new(self.http_client.clone(), api_key, base_url)))
                }
            },
            _ => anyhow::bail!("Unsupported provider type: {}", provider.provider_type),
        }
    }
}
