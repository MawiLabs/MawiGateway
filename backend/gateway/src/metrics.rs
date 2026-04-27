//! Prometheus metric registry + per-metric `lazy_static` handles.
//!
//! Each `register_*_with_registry!` macro returns `Result<Metric, Error>`;
//! we `.expect()` because the only error case is "name already in
//! registry," which `lazy_static` makes unreachable in practice (each
//! handle runs its initialiser once). The panic path remains as a
//! defensive belt: if a future refactor (custom test registry, hot
//! reload, etc.) ever hits a duplicate, we want to crash loudly at
//! startup rather than silently lose a metric — see #35.
//!
//! The expect messages used to be identical copy-pastes naming
//! `HTTP_REQUESTS_TOTAL` for every metric, which was actively
//! misleading on failure. They now describe the real failure mode.

use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec_with_registry, register_histogram_with_registry,
    register_int_counter_vec_with_registry, register_int_counter_with_registry,
    register_int_gauge_with_registry, Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

lazy_static! {
    pub static ref METRICS_REGISTRY: Arc<Registry> = Arc::new(Registry::new());

    // ============ PERFORMANCE METRICS ============

    /// Total HTTP requests
    pub static ref HTTP_REQUESTS_TOTAL: IntCounter = register_int_counter_with_registry!(
        Opts::new("http_requests_total", "Total HTTP requests"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// HTTP errors
    pub static ref HTTP_REQUESTS_ERRORS: IntCounter = register_int_counter_with_registry!(
        Opts::new("http_requests_errors_total", "Total HTTP request errors"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Request latency histogram (global, no labels — kept for backwards compat).
    pub static ref REQUEST_DURATION: Histogram = register_histogram_with_registry!(
       HistogramOpts::new("http_request_duration_seconds_global", "HTTP request latency (all routes)")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Per-route request count, labelled by route + method + status. The route
    /// label is the matched path with high-cardinality segments collapsed
    /// (UUIDs, numeric IDs become `:id`) to keep Prometheus cardinality bounded.
    pub static ref HTTP_REQUESTS_BY_ROUTE: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("http_requests_total_by_route", "HTTP requests by route, method, status"),
        &["route", "method", "status"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUESTS_BY_ROUTE metric");

    /// Per-route latency histogram, same labels as HTTP_REQUESTS_BY_ROUTE.
    /// SRE buckets: from 5ms (cache hit) to 30s (slow video gen).
    pub static ref HTTP_REQUEST_DURATION_BY_ROUTE: HistogramVec = register_histogram_vec_with_registry!(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request latency by route, method, status")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]),
        &["route", "method", "status"],
        METRICS_REGISTRY.clone()
    ).expect("Failed to register HTTP_REQUEST_DURATION_BY_ROUTE metric");

    /// In-flight requests (gauge)
    pub static ref REQUESTS_IN_FLIGHT: IntGauge = register_int_gauge_with_registry!(
        Opts::new("http_requests_in_flight", "Number of HTTP requests currently being processed"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Database query latency by operation
    pub static ref DB_QUERY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
        HistogramOpts::new("db_query_duration_seconds", "Database query latency")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
        &["operation"],
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Provider API call latency by provider
    pub static ref PROVIDER_CALL_DURATION: HistogramVec = register_histogram_vec_with_registry!(
        HistogramOpts::new("provider_call_duration_seconds", "Provider API call latency")
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]),
        &["provider", "model"],
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    // ============ ERROR TRACKING ============

    /// Errors by type
    pub static ref ERRORS_BY_TYPE: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("errors_total", "Errors by type"),
        &["error_type"],
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Database errors
    pub static ref DB_ERRORS: IntCounter = register_int_counter_with_registry!(
        Opts::new("db_errors_total", "Total database errors"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Provider API errors by provider
    pub static ref PROVIDER_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("provider_errors_total", "Provider API errors"),
        &["provider", "error_type"],
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    // ============ AVAILABILITY METRICS ============

    /// Service health status (1 = healthy, 0 = unhealthy)
    pub static ref SERVICE_HEALTH: IntGauge = register_int_gauge_with_registry!(
        Opts::new("service_health_status", "Service health status (1=healthy, 0=unhealthy)"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Database connection pool - active connections
    pub static ref DB_CONNECTIONS_ACTIVE: IntGauge = register_int_gauge_with_registry!(
        Opts::new("db_connections_active", "Active database connections"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    /// Database connection pool - idle connections
    pub static ref DB_CONNECTIONS_IDLE: IntGauge = register_int_gauge_with_registry!(
        Opts::new("db_connections_idle", "Idle database connections"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    // ============ CIRCUIT BREAKER METRICS ============

    pub static ref CIRCUIT_BREAKER_TRIPS: IntCounter = register_int_counter_with_registry!(
        Opts::new("circuit_breaker_trips_total", "Total circuit breaker trips"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    pub static ref CIRCUIT_BREAKER_OPEN: IntGauge = register_int_gauge_with_registry!(
        Opts::new("circuit_breaker_open_count", "Number of currently open circuit breakers"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    // ============ CACHE METRICS ============

    pub static ref CACHE_HITS: IntCounter = register_int_counter_with_registry!(
        Opts::new("cache_hits_total", "Total cache hits"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    pub static ref CACHE_MISSES: IntCounter = register_int_counter_with_registry!(
        Opts::new("cache_misses_total", "Total cache misses"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    // ============ WORKER/QUEUE METRICS ============

    pub static ref LOG_BUFFER_DEPTH: IntGauge = register_int_gauge_with_registry!(
        Opts::new("log_buffer_depth", "Current log buffer size"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    pub static ref LOG_DROPS: IntCounter = register_int_counter_with_registry!(
        Opts::new("log_drops_total", "Total dropped log entries"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    pub static ref QUOTA_WORKER_QUEUE_DEPTH: IntGauge = register_int_gauge_with_registry!(
        Opts::new("quota_worker_queue_depth", "Quota worker queue depth"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    // ============ MODEL EXECUTION METRICS ============

    pub static ref MODEL_REQUESTS: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("model_requests_total", "Total model execution requests"),
        &["service", "model"],
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    pub static ref MODEL_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
        Opts::new("model_errors_total", "Total model execution errors"),
        &["service", "model", "error_type"],
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");

    pub static ref FAILOVER_COUNT: IntCounter = register_int_counter_with_registry!(
        Opts::new("failover_total", "Total failover attempts"),
        METRICS_REGISTRY.clone()
    ).expect("metrics registration failed — duplicate metric name in METRICS_REGISTRY");
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
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("Failed to encode Prometheus metrics");
    String::from_utf8(buffer).expect("Prometheus metrics contained invalid UTF-8")
}

/// Whether the `/metrics` endpoint should be mounted.
///
/// On by default. To suppress (e.g. behind an unauthenticated edge), set
/// `DISABLE_METRICS=true`. The legacy `ENABLE_METRICS=true` opt-in is also
/// honoured so existing deployments don't suddenly lose the endpoint.
pub fn metrics_enabled() -> bool {
    if std::env::var("DISABLE_METRICS")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return false;
    }
    // Legacy: if the operator explicitly set ENABLE_METRICS=false, respect it.
    if let Ok(v) = std::env::var("ENABLE_METRICS") {
        return v.eq_ignore_ascii_case("true");
    }
    true
}
