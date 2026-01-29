-- Create request logs table
CREATE TABLE IF NOT EXISTS request_logs (
    id TEXT PRIMARY KEY,
    virtual_key_id TEXT,             -- Optional: Link to virtual key if used
    service_name TEXT NOT NULL,      -- Service attempted (e.g. "chat-general")
    model_id TEXT NOT NULL,          -- Actual model used
    provider_type TEXT,              -- "openai", "azure", etc.
    
    -- Metrics
    tokens_prompt INTEGER,
    tokens_completion INTEGER,
    tokens_total INTEGER,
    latency_ms INTEGER NOT NULL,
    cost_usd REAL,                   -- Estimated cost
    
    -- Status
    status TEXT NOT NULL,            -- "success", "error"
    error_message TEXT,
    failover_count INTEGER DEFAULT 0, -- How many failovers occurred before this attempt
    
    created_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    user_id TEXT
);

-- Indexes for analytics performance
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_request_logs_service ON request_logs(service_name);
CREATE INDEX IF NOT EXISTS idx_request_logs_model ON request_logs(model_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_status ON request_logs(status);
CREATE INDEX IF NOT EXISTS idx_request_logs_user_id ON request_logs(user_id);
