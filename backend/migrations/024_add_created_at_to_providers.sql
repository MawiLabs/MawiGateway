-- Migration 024: Add created_at and updated_at to providers table
-- Missing columns causing "no column found for name: created_at" error

ALTER TABLE providers ADD COLUMN IF NOT EXISTS created_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT);
ALTER TABLE providers ADD COLUMN IF NOT EXISTS updated_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT);
