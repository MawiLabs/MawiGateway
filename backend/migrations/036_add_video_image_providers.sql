-- Migration 036: previously seeded image/video/audio providers + flagship
-- models so the new adapter code had something to talk to out of the box.
--
-- That was wrong: providers are user-managed (the dashboard scopes its
-- listings by user_id), so seeded system rows showed up in the DB without
-- showing up in the UI — confusing and unwanted. We now leave provider
-- registration to the admin UI / mawigateway.yaml / API entirely.
--
-- This migration is intentionally empty so anyone who hasn't run the
-- previous version still gets a clean DB. Environments that already ran
-- the seeding version of 036 keep those rows; a follow-up cleanup
-- migration can delete them once we've decided on the right semantics.
--
-- The new provider_type strings (runway, kling, lumaai, pika, minimax,
-- bytedance, hume) don't need a migration — provider_type is a free TEXT
-- column. The factory in executor.rs already knows how to instantiate
-- adapters for them.

SELECT 1; -- no-op so sqlx accepts the file
