-- Migration: Add agentic service support
-- Created: 2026-01-06

-- Add agentic-specific columns to services table
-- Add agentic-specific columns to services table
-- ALTER TABLE services ADD COLUMN system_prompt TEXT;
-- ALTER TABLE services ADD COLUMN max_iterations INTEGER DEFAULT 10;
SELECT 1;

-- Create agentic_tools table for tool configuration
CREATE TABLE IF NOT EXISTS agentic_tools (
    id TEXT PRIMARY KEY,
    service_name TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    tool_type TEXT NOT NULL CHECK (tool_type IN ('model', 'service', 'image_generation', 'video_generation', 'text_to_speech', 'speech_to_text')),
    target_id TEXT NOT NULL,
    parameters_schema TEXT,  -- JSON Schema for parameters
    position INTEGER DEFAULT 0,
    created_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    FOREIGN KEY (service_name) REFERENCES services(name) ON DELETE CASCADE,
    UNIQUE(service_name, name)
);

-- Index for fast lookup by service
CREATE INDEX IF NOT EXISTS idx_agentic_tools_service ON agentic_tools(service_name);

-- Ensure planner_model_id column exists (may already exist from earlier work)
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we handle this gracefully
-- This will error if column exists, which is fine for migration scripts
-- ALTER TABLE services ADD COLUMN planner_model_id TEXT;
