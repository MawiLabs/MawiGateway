-- Refactor User Tiers from A/B/C to meaningful names

-- Update users table
UPDATE users SET tier = 'community' WHERE tier = 'A';
UPDATE users SET tier = 'team' WHERE tier = 'B';
UPDATE users SET tier = 'enterprise' WHERE tier = 'C';

-- Update organizations table
UPDATE organizations SET tier = 'community' WHERE tier = 'A';
UPDATE organizations SET tier = 'team' WHERE tier = 'B';
UPDATE organizations SET tier = 'enterprise' WHERE tier = 'C';
