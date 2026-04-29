-- Service aliases (Tier-2 #8 follow-up).
--
-- Operators can declare alternate names that route to a service —
-- explicit, opt-in, per service. This is the right answer for "OpenAI
-- drop-in compat" without giving up the service abstraction:
--
--   service "text-default" with aliases [gpt-4o, gpt-4-turbo, claude-3-5-sonnet]
--
-- An OpenAI client sending `service: "gpt-4o"` (or `model: "gpt-4o"`
-- via a future SDK helper) routes to text-default — picking up the
-- pool, the strategy, the cache, the budget, all of it. The operator
-- explicitly listed `gpt-4o` as an alias, so this is consent, not a
-- silent fallback.

ALTER TABLE services
    ADD COLUMN aliases TEXT[] NOT NULL DEFAULT '{}';

-- GIN index so reverse lookup (`WHERE 'gpt-4o' = ANY(aliases)`) is
-- O(log n) instead of a full scan. Critical because every chat call
-- that uses an alias goes through this query.
CREATE INDEX services_aliases_gin_idx ON services USING GIN (aliases);

-- Uniqueness across (name, alias) namespace. We don't want two
-- services to claim the same alias — that would make routing
-- non-deterministic. The constraint is enforced via a partial unique
-- index on a derived view: every alias must be unique across all
-- services, AND must not collide with any service's `name`.
--
-- Postgres can't enforce both invariants in a single constraint, so
-- we add a per-row trigger. Cheaper, and gives a clear error message.
CREATE OR REPLACE FUNCTION check_service_alias_uniqueness()
RETURNS TRIGGER AS $$
DECLARE
    conflict_name TEXT;
    conflict_alias TEXT;
BEGIN
    -- Reject aliases that collide with another service's canonical name.
    IF NEW.aliases IS NOT NULL AND array_length(NEW.aliases, 1) > 0 THEN
        SELECT name INTO conflict_name
        FROM services
        WHERE name = ANY(NEW.aliases)
          AND name <> NEW.name
        LIMIT 1;
        IF conflict_name IS NOT NULL THEN
            RAISE EXCEPTION 'alias % collides with existing service name %', conflict_name, conflict_name
                USING ERRCODE = 'unique_violation';
        END IF;

        -- Reject aliases that another service already claims.
        SELECT s.name, a.alias INTO conflict_name, conflict_alias
        FROM services s,
             unnest(s.aliases) AS a(alias)
        WHERE a.alias = ANY(NEW.aliases)
          AND s.name <> NEW.name
        LIMIT 1;
        IF conflict_alias IS NOT NULL THEN
            RAISE EXCEPTION 'alias % already used by service %', conflict_alias, conflict_name
                USING ERRCODE = 'unique_violation';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER services_alias_uniqueness
    BEFORE INSERT OR UPDATE OF aliases, name ON services
    FOR EACH ROW
    EXECUTE FUNCTION check_service_alias_uniqueness();
