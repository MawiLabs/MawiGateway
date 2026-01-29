use prometheus::{
    IntCounter, IntGauge, Histogram, HistogramOpts, HistogramVec, IntCounterVec, Opts,
    Registry, TextEncoder, Encoder,
    register_int_counter_with_registry, register_int_gauge_with_registry, 
    register_histogram_with_registry, register_histogram_vec_with_registry,
    register_int_counter_vec_with_registry,
};
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref METRICS_REGISTRY: Arc<Registry> = Arc::new(Registry::new());
    
    // ============ PERFORMANCE METRICS ============
    
    /// Total HTTP requests
    pub static ref HTTP_REQUESTS_TOTAL: IntCounter = register_int_counter_with_registry!(
        Opts::new("http_requests_total", "Total HTTP requests"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// HTTP errors
    pub static ref HTTP_REQUESTS_ERRORS: IntCounter = register_int_counter_with_registry!(
        Opts::new("http_requests_errors_total", "Total HTTP request errors"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Request latency histogram
    pub static ref REQUEST_DURATION: Histogram = register_histogram_with_registry!(
       HistogramOpts::new("http_request_duration_seconds", "HTTP request latency")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// In-flight requests (gauge)
    pub static ref REQUESTS_IN_FLIGHT: IntGauge = register_int_gauge_with_registry!(
        Opts::new("http_requests_in_flight", "Number of HTTP requests currently being processed"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Database query latency by operation
    pub static ref DB_QUERY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
        HistogramOpts::new("db_query_duration_seconds", "Database query latency")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
        &["operation"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Provider API call latency by provider
    pub static ref PROVIDER_CALL_DURATION: HistogramVec = register_histogram_vec_with_registry!(
        HistogramOpts::new("provider_call_duration_seconds", "Provider API call latency")
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]),
        &["provider", "model"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    // ============ ERROR TRACKING ============
    
    /// Errors by type
    pub static ref ERRORS_BY_TYPE: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("errors_total", "Errors by type"),
        &["error_type"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Database errors
    pub static ref DB_ERRORS: IntCounter = register_int_counter_with_registry!(
        Opts::new("db_errors_total", "Total database errors"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Provider API errors by provider
    pub static ref PROVIDER_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("provider_errors_total", "Provider API errors"),
        &["provider", "error_type"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    // ============ AVAILABILITY METRICS ============
    
    /// Service health status (1 = healthy, 0 = unhealthy)
    pub static ref SERVICE_HEALTH: IntGauge = register_int_gauge_with_registry!(
        Opts::new("service_health_status", "Service health status (1=healthy, 0=unhealthy)"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Database connection pool - active connections
    pub static ref DB_CONNECTIONS_ACTIVE: IntGauge = register_int_gauge_with_registry!(
        Opts::new("db_connections_active", "Active database connections"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    /// Database connection pool - idle connections
    pub static ref DB_CONNECTIONS_IDLE: IntGauge = register_int_gauge_with_registry!(
        Opts::new("db_connections_idle", "Idle database connections"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    // ============ CIRCUIT BREAKER METRICS ============
    
    pub static ref CIRCUIT_BREAKER_TRIPS: IntCounter = register_int_counter_with_registry!(
        Opts::new("circuit_breaker_trips_total", "Total circuit breaker trips"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    pub static ref CIRCUIT_BREAKER_OPEN: IntGauge = register_int_gauge_with_registry!(
        Opts::new("circuit_breaker_open_count", "Number of currently open circuit breakers"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    // ============ CACHE METRICS ============
    
    pub static ref CACHE_HITS: IntCounter = register_int_counter_with_registry!(
        Opts::new("cache_hits_total", "Total cache hits"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    pub static ref CACHE_MISSES: IntCounter = register_int_counter_with_registry!(
        Opts::new("cache_misses_total", "Total cache misses"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    // ============ WORKER/QUEUE METRICS ============
    
    pub static ref LOG_BUFFER_DEPTH: IntGauge = register_int_gauge_with_registry!(
        Opts::new("log_buffer_depth", "Current log buffer size"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    pub static ref LOG_DROPS: IntCounter = register_int_counter_with_registry!(
        Opts::new("log_drops_total", "Total dropped log entries"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    pub static ref QUOTA_WORKER_QUEUE_DEPTH: IntGauge = register_int_gauge_with_registry!(
        Opts::new("quota_worker_queue_depth", "Quota worker queue depth"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    // ============ MODEL EXECUTION METRICS ============
    
    pub static ref MODEL_REQUESTS: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("model_requests_total", "Total model execution requests"),
        &["service", "model"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    pub static ref MODEL_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("model_errors_total", "Total model execution errors"),
        &["service", "model", "error_type"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
    
    pub static ref FAILOVER_COUNT: IntCounter = register_int_counter_with_registry!(
        Opts::new("failover_total", "Total failover attempts"),
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_TOTAL metric");
}

/// Get metrics as Prometheus-formatted text
pub fn gather_metrics() -> String {
    // Force initialization of all lazy_static metrics
    let _ = HTTP_REQUESTS_TOTAL.get();
    let _ = HTTP_REQUESTS_ERRORS.get();
    let _ = REQUEST_DURATION.get_sample_count();
    let _ = REQUESTS_IN_FLIGHT.get();
    let _ = SERVICE_HEALTH.get();
    let _ = DB_CONNECTIONS_ACTIVE.get();
    let _ = DB_CONNECTIONS_IDLE.get();
    let _ = DB_ERRORS.get();
    let _ = CIRCUIT_BREAKER_TRIPS.get();
    let _ = CIRCUIT_BREAKER_OPEN.get();
    let _ = CACHE_HITS.get();
    let _ = CACHE_MISSES.get();
    let _ = LOG_BUFFER_DEPTH.get();
    let _ = LOG_DROPS.get();
    let _ = QUOTA_WORKER_QUEUE_DEPTH.get();
    let _ = FAILOVER_COUNT.get();
    
    let encoder = TextEncoder::new();
    let metric_families = METRICS_REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)
        .expect("Failed to encode Prometheus metrics");
    String::from_utf8(buffer)
        .expect("Prometheus metrics contained invalid UTF-8")
}

/// Check if metrics are enabled via environment variable
pub fn metrics_enabled() -> bool {
    std::env::var("ENABLE_METRICS")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true"
}
