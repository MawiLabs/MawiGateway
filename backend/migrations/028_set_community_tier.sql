-- Migration: Set all users to community tier
-- This migration updates existing users to use the new "community" tier

-- Drop restrictive check constraints first
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_tier_check;
ALTER TABLE organizations DROP CONSTRAINT IF EXISTS organizations_tier_check;

-- Update all existing users to community tier
UPDATE users SET tier = 'community' WHERE tier IN ('A', 'B', 'C') OR tier IS NULL;
UPDATE organizations SET tier = 'community' WHERE tier IS NULL;

-- Set default tier for new users
ALTER TABLE users ALTER COLUMN tier SET DEFAULT 'community';
ALTER TABLE organizations ALTER COLUMN tier SET DEFAULT 'community';
