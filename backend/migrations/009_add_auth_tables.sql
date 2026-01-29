-- Migration 009: Add Authentication and Access Control
-- Created: 2025-12-17
-- Purpose: Implement tier-based access control with users, orgs, and quotas

-- Organizations table
CREATE TABLE IF NOT EXISTS organizations (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  tier TEXT NOT NULL CHECK(tier IN ('A', 'B', 'C')),
  monthly_quota_usd DOUBLE PRECISION NOT NULL,
  current_usage_usd DOUBLE PRECISION DEFAULT 0.0,
  quota_reset_at BIGINT NOT NULL,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL
);

-- Users table
CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT,  -- bcrypt hash, nullable for OAuth users
  name TEXT,
  org_id TEXT,
  tier TEXT NOT NULL CHECK(tier IN ('A', 'B', 'C')),
  monthly_quota_usd DOUBLE PRECISION NOT NULL,
  current_usage_usd DOUBLE PRECISION DEFAULT 0.0,
  quota_reset_at BIGINT NOT NULL,
  is_admin BOOLEAN DEFAULT FALSE,  -- Org admin
  is_free_tier BOOLEAN DEFAULT TRUE,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL,
  last_login_at BIGINT,
  FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE SET NULL
);

-- Sessions (for cookie-based auth)
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,  -- Random token
  user_id TEXT NOT NULL,
  expires_at BIGINT NOT NULL,
  created_at BIGINT NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- API Keys (for programmatic access)
CREATE TABLE IF NOT EXISTS api_keys (
  id TEXT PRIMARY KEY,  -- "sk_live_..." format
  user_id TEXT NOT NULL,
  name TEXT,  -- User-friendly name
  key_hash TEXT NOT NULL,  -- SHA-256 hash
  last_used_at BIGINT,
  created_at BIGINT NOT NULL,
  expires_at BIGINT,  -- Nullable (never expires)
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Add tier requirements to models
-- ALTER TABLE models ADD COLUMN tier_required TEXT DEFAULT 'A' CHECK(tier_required IN ('A', 'B', 'C'));
-- ALTER TABLE models ADD COLUMN worker_type TEXT DEFAULT 'text' CHECK(worker_type IN ('text', 'stt', 'tts', 'video_gen', 'video_understand'));

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_org_id ON users(org_id);
CREATE INDEX IF NOT EXISTS idx_users_tier ON users(tier);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_models_tier_required ON models(tier_required);

-- Create default admin user with UUID
-- Default admin removed in favor of "First User is Admin" policy
-- The first user to register will automatically be assigned admin privileges.

-- Update existing models with tier requirements
-- Text models (Tier A)
UPDATE models SET tier_required = 'A', worker_type = 'text' WHERE modality = 'text';

-- Audio models (Tier B) - will be added later
-- UPDATE models SET tier_required = 'B', worker_type = 'stt' WHERE name LIKE '%whisper%';
-- UPDATE models SET tier_required = 'B', worker_type = 'tts' WHERE name LIKE '%tts%';

-- Video models (Tier C) - will be added later
-- UPDATE models SET tier_required = 'C', worker_type = 'video_gen' WHERE name LIKE '%sora%';
