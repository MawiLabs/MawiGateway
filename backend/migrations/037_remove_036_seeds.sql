-- ============================================================================
-- Migration 037 — clean up seed rows that the FIRST version of 036 inserted.
-- ============================================================================
--
-- Why this exists. Migration 036 (see file header) shipped twice within
-- a day:
--
--   v1: INSERT INTO providers + models for 8 video/image/audio
--       providers, with NULL user_id (system rows).
--   v2: rewritten to a no-op `SELECT 1`.
--
-- Environments that pulled main between v1 and v2 already have those
-- rows; the no-op version won't remove them by itself. This migration
-- is the explicit cleanup.
--
-- ============================================================================
-- SAFETY: the WHERE clauses match ONLY the original seed shape
-- ============================================================================
--
--   user_id IS NULL                ← no admin claimed the row via UI
--   AND (api_key IS NULL OR '')    ← no admin pasted a real credential
--
-- An admin who manually re-added one of these provider ids through
-- the dashboard / yaml / API would have set user_id to their own id
-- and (typically) an encrypted api_key — those rows do NOT match
-- and are preserved.
--
-- Models cascade naturally because `models.provider_id REFERENCES
-- providers(id) ON DELETE CASCADE` (see 001_initial_schema), but we
-- delete the model rows explicitly first so the migration's intent is
-- visible without chasing the FK to the providers table.
--
-- ============================================================================
-- Idempotent. Re-running has no effect once the seed rows are gone.
-- Safe on environments that ran v2 of 036 directly (DELETE matches 0 rows).

DELETE FROM models
 WHERE id IN (
    'xai-grok-imagine-v1',
    'runway-gen-4',
    'runway-gen-4-5',
    'kling-v1-5',
    'kling-v2',
    'luma-ray-2',
    'luma-ray-flash-2',
    'pika-2-2',
    'minimax-hailuo-02',
    'minimax-abab-6-5-chat',
    'bytedance-seedance-1-0-pro',
    'bytedance-seedance-1-0-lite',
    'hume-octave',
    'hume-evi-3'
 )
 AND user_id IS NULL
 AND (api_key IS NULL OR api_key = '');

DELETE FROM providers
 WHERE id IN (
    'xai',
    'runway',
    'kling',
    'lumaai',
    'pika',
    'minimax',
    'bytedance',
    'hume'
 )
 AND user_id IS NULL
 AND (api_key IS NULL OR api_key = '');
