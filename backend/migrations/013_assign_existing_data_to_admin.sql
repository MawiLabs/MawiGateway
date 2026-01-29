-- Assign all existing data to the admin user (default-user)
-- This ensures existing providers, services, models belong to the admin account

-- Update providers
UPDATE providers SET user_id = 'default-user' WHERE user_id IS NULL;

-- Update services
UPDATE services SET user_id = 'default-user' WHERE user_id IS NULL;

-- Update models
UPDATE models SET user_id = 'default-user' WHERE user_id IS NULL;

-- Update request_logs (if any exist)
UPDATE request_logs SET user_id = 'default-user' WHERE user_id IS NULL;
