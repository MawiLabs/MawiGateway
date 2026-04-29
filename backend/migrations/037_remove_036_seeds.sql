-- Migration 037: clean up seed rows that the original version of
-- migration 036 inserted before we made it a no-op.
--
-- Environments that ran the seeding version of 036 already have these
-- rows; environments that ran the no-op version (post-fix) won't see
-- them. Either way this migration is safe to apply: rows are only
-- removed when they look exactly like the seed (NULL user_id, NULL
-- api_key) so we never touch a provider an admin actually configured
-- through the UI / yaml / API.
--
-- Models cascade because models.provider_id has ON DELETE CASCADE
-- (see 001_initial_schema), but we delete by id explicitly so the
-- intent is visible in the migration history.

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
