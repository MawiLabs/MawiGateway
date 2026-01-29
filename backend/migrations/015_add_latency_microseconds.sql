-- Add latency_us (microseconds) column for better precision
ALTER TABLE request_logs ADD COLUMN IF NOT EXISTS latency_us BIGINT;
-- SELECT 1;

-- Migrate existing data: convert ms to microseconds
UPDATE request_logs SET latency_us = latency_ms * 1000 WHERE latency_us IS NULL;
