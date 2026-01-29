-- Migration 027: Add created_by column to models table
-- Fixes "no column found for name: created_by" error in SQLx FromRow mapping

ALTER TABLE models ADD COLUMN IF NOT EXISTS created_by TEXT;
