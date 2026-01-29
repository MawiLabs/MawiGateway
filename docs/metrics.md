# Prometheus Metrics Usage Guide

## Quick Start

### 1. Enable Metrics

Set the environment variable:
```bash
export ENABLE_METRICS=true
```

Or in your `.env` file:
```
ENABLE_METRICS=true
```

### 2. Start the Gateway

```bash
cd backend
cargo run --bin gateway
```

You should see:
```
📊 Metrics enabled at /metrics
```

### 3. Access Metrics

```bash
curl http://localhost:8030/metrics
```

## Available Metrics

### Request Metrics
- `http_requests_total` - Total HTTP requests
- `http_requests_errors_total` - Total HTTP errors
- `http_request_duration_seconds` - Request latency histogram

### Circuit Breaker Metrics
- `circuit_breaker_trips_total` - Total circuit breaker trips
- `circuit_breaker_open_count` - Currently open circuit breakers

### Cache Metrics
- `cache_hits_total` - Total cache hits
- `cache_misses_total` - Total cache misses

### Worker Metrics
- `log_buffer_depth` - Current log buffer size
- `log_drops_total` - Total dropped log entries

### Model Metrics
- `model_requests_total` - Total model execution requests
- `model_errors_total` - Total model execution errors
- `failover_total` - Total failover attempts

## Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'mawi-gateway'
    static_configs:
      - targets: ['localhost:8030']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

## Grafana Dashboard

Example queries:

### Request Rate
```promql
rate(http_requests_total[5m])
```

### Error Rate
```promql
rate(http_requests_errors_total[5m]) / rate(http_requests_total[5m])
```

### P95 Latency
```promql
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

### Circuit Breaker Status
```promql
circuit_breaker_open_count
```

## Production Considerations

- **Default**: Metrics are DISABLED (`ENABLE_METRICS=false`)
- **Why**: Minor performance overhead from atomic operations
- **When to enable**: Always in staging/production for observability
- **Security**: No authentication on `/metrics` - use firewall/networking to restrict access
