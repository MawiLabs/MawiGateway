-- Optimizing lookup performance for high-traffic paths

-- Accelerate service -> model resolution
CREATE INDEX IF NOT EXISTS idx_service_models_service_lookup ON service_models(service_name);

-- Accelerate latency stats calculation (used in routing)
-- Covers: WHERE model_id = ? AND status = 'success' AND created_at > ?
CREATE INDEX IF NOT EXISTS idx_request_logs_latency_stats ON request_logs(model_id, status, created_at);

-- Accelerate user lookups
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
