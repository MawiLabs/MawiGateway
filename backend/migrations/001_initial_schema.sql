-- Migration 001: Initial Schema Snapshot
-- Restoring missing base tables required for subsequent migrations

-- Providers
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    api_endpoint TEXT,
    api_version TEXT,
    api_key TEXT,
    description TEXT,
    icon_url TEXT,
    user_id TEXT
);

-- Models
CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    modality TEXT,
    description TEXT,
    api_endpoint TEXT,
    api_version TEXT,
    api_key TEXT,
    created_at BIGINT,
    tier_required TEXT DEFAULT 'A',
    worker_type TEXT DEFAULT 'text',
    user_id TEXT,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

-- Services
CREATE TABLE IF NOT EXISTS services (
    name TEXT PRIMARY KEY,
    service_type TEXT NOT NULL, -- 'POOL' or 'AGENTIC'
    description TEXT,
    strategy TEXT DEFAULT 'weighted',
    guardrails TEXT DEFAULT '[]', -- JSON array
    user_id TEXT,
    
    -- Agentic specific
    planner_model_id TEXT,
    system_prompt TEXT,
    max_iterations INTEGER,
    
    created_at BIGINT,
    
    -- Pool specific capabilities
    pool_type TEXT,
    input_modalities TEXT DEFAULT '["text"]',
    output_modalities TEXT DEFAULT '["text"]'
);

-- Service Models Assignment
CREATE TABLE IF NOT EXISTS service_models (
    service_name TEXT NOT NULL,
    model_id TEXT NOT NULL,
    modality TEXT,
    position INTEGER DEFAULT 0,
    weight INTEGER DEFAULT 100,
    
    -- RTCROS Components
    rtcros_role TEXT,
    rtcros_task TEXT,
    rtcros_context TEXT,
    rtcros_reasoning TEXT,
    rtcros_output TEXT,
    rtcros_stop TEXT,
    
    PRIMARY KEY (service_name, model_id),
    FOREIGN KEY (service_name) REFERENCES services(name) ON DELETE CASCADE,
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_models_provider_id ON models(provider_id);
CREATE INDEX IF NOT EXISTS idx_service_models_service_name ON service_models(service_name);
CREATE INDEX IF NOT EXISTS idx_providers_user_id ON providers(user_id);
CREATE INDEX IF NOT EXISTS idx_models_user_id ON models(user_id);
CREATE INDEX IF NOT EXISTS idx_services_user_id ON services(user_id);
