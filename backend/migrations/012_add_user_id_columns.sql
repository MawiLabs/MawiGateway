-- Migration 012: Add user_id columns
-- ALREADY APPLIED via initial schema snapshot manually for providers, services, models.
-- EXCEPT request_logs was missing it.

-- Add user_id to providers table
-- ALTER TABLE providers ADD COLUMN user_id TEXT;

-- Add user_id to services table  
-- ALTER TABLE services ADD COLUMN user_id TEXT;

-- Add user_id to models table
-- ALTER TABLE models ADD COLUMN user_id TEXT;

-- Add user_id to request_logs table (if it exists)
-- ALTER TABLE request_logs ADD COLUMN user_id TEXT;

-- Create indexes for better query performance
-- CREATE INDEX IF NOT EXISTS idx_providers_user_id ON providers(user_id);
-- CREATE INDEX IF NOT EXISTS idx_services_user_id ON services(user_id);
-- CREATE INDEX IF NOT EXISTS idx_models_user_id ON models(user_id);
-- CREATE INDEX IF NOT EXISTS idx_request_logs_user_id ON request_logs(user_id);
