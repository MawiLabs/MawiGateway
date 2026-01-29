-- Model Health Tracking Table
CREATE TABLE IF NOT EXISTS model_health (
    model_id TEXT PRIMARY KEY,
    is_healthy INTEGER NOT NULL DEFAULT 1,  -- 1 = healthy, 0 = unhealthy
    last_check INTEGER NOT NULL,            -- Unix timestamp
    response_time_ms INTEGER,               -- Last health check latency
    consecutive_failures INTEGER DEFAULT 0, -- Track failure streak
    last_error TEXT,                        -- Last error message if any
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE
);

-- Indexes for efficient health queries
CREATE INDEX IF NOT EXISTS idx_model_health_status ON model_health(is_healthy);
CREATE INDEX IF NOT EXISTS idx_model_health_last_check ON model_health(last_check);
