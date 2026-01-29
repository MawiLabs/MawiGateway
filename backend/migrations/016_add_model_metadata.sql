-- Phase 1 Migration: Add Model Metadata for Intelligent Routing
-- This adds pricing, performance metrics, and tier information to models table

-- Pricing columns
ALTER TABLE models ADD COLUMN cost_per_1k_tokens REAL DEFAULT 0.0;
ALTER TABLE models ADD COLUMN cost_per_1k_input_tokens REAL DEFAULT 0.0;
ALTER TABLE models ADD COLUMN cost_per_1k_output_tokens REAL DEFAULT 0.0;
ALTER TABLE models ADD COLUMN tier TEXT DEFAULT 'standard';

-- Performance metrics
ALTER TABLE models ADD COLUMN avg_latency_ms INTEGER DEFAULT 0;
ALTER TABLE models ADD COLUMN avg_ttft_ms INTEGER DEFAULT 0; -- Time to first token
ALTER TABLE models ADD COLUMN max_tps INTEGER DEFAULT 0; -- Tokens per second

-- Service models enhancements
ALTER TABLE service_models ADD COLUMN priority INTEGER DEFAULT 1;
ALTER TABLE service_models ADD COLUMN enabled BOOLEAN DEFAULT TRUE; -- Circuit breaker state
SELECT 1;
