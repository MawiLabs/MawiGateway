-- Comprehensive Model Pricing Database
-- Prices are in USD per million tokens (input/output) or per unit as specified
-- Last updated: December 2024

-- OpenAI Models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.150,  -- $150/1M = $0.150/1K
    cost_per_1k_output_tokens = 0.600, -- $600/1M = $0.600/1K
    tier = 'premium'
WHERE name LIKE '%gpt-4%' AND name NOT LIKE '%gpt-4o%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.0025, -- $2.50/1M
    cost_per_1k_output_tokens = 0.010, -- $10/1M
    tier = 'standard'
WHERE name LIKE '%gpt-4o%' OR name LIKE '%gpt-4-turbo%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.0005, -- $0.50/1M
    cost_per_1k_output_tokens = 0.0015, -- $1.50/1M
    tier = 'standard'
WHERE name LIKE '%gpt-3.5%';

-- Anthropic Claude Models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.005, -- $5/1M
    cost_per_1k_output_tokens = 0.025, -- $25/1M
    tier = 'premium'
WHERE name LIKE '%claude%opus%4.5%' OR name LIKE '%claude-opus-4.5%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.003, -- $3/1M
    cost_per_1k_output_tokens = 0.015, -- $15/1M
    tier = 'premium'
WHERE name LIKE '%claude%sonnet%4.5%' OR name LIKE '%claude-4.5-sonnet%' OR name LIKE '%claude-sonnet-4%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.001, -- $1/1M
    cost_per_1k_output_tokens = 0.005, -- $5/1M
    tier = 'standard'
WHERE name LIKE '%claude%haiku%' OR name LIKE '%claude-haiku%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.00163, -- $1.63/1M
    cost_per_1k_output_tokens = 0.00551, -- $5.51/1M
    tier = 'standard'
WHERE name LIKE '%claude-instant%';

-- Google Gemini Models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.00125, -- Gemini 2.5 Pro free tier, paid varies
    cost_per_1k_output_tokens = 0.005,
    tier = 'premium'
WHERE name LIKE '%gemini-2.5-pro%' OR name LIKE '%gemini-pro%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.000075, -- Gemini 2.0/2.5 Flash - very cheap
    cost_per_1k_output_tokens = 0.0003,
    tier = 'standard'
WHERE name LIKE '%gemini%flash%' OR name LIKE '%gemini-2.0-flash%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.0, -- Free embedding
    cost_per_1k_output_tokens = 0.0,
    tier = 'free'
WHERE name LIKE '%gemini-embedding%';

-- Google Video Models (Veo) - per video generation
-- Note: These are charged per generation, not per token
-- Storing as cost per 1k "tokens" where 1 generation = 1000 tokens for simplicity
UPDATE models SET 
    cost_per_1k_tokens = 0.040, -- $0.04 per generation  = $40 per 1000 generations
    tier = 'premium'
WHERE name LIKE '%veo-3.1%' OR name LIKE '%veo-3%' OR name LIKE '%veo-2%';

-- Google Image Models (Imagen) - per image generation
UPDATE models SET 
    cost_per_1k_tokens = 0.020, -- $0.02 per image = $20 per 1000 images
    tier = 'standard'
WHERE name LIKE '%imagen-%';

-- DeepSeek Models (very cheap!)
UPDATE models SET 
    cost_per_1k_input_tokens = 0.00014, -- $0.14/1M
    cost_per_1k_output_tokens = 0.00028, -- $0.28/1M
    tier = 'free'
WHERE name LIKE '%deepseek-chat%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.00027, -- $0.27/1M
    cost_per_1k_output_tokens = 0.0011, -- $1.1/1M
    tier = 'free'
WHERE name LIKE '%deepseek-reasoner%' OR name LIKE '%deepseek-r1%';

-- xAI Grok Models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.005, -- $5/1M
    cost_per_1k_output_tokens = 0.015, -- $15/1M
    tier = 'premium'
WHERE name LIKE '%grok%';

-- Mistral Models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.002, -- Mistral Large
    cost_per_1k_output_tokens = 0.006,
    tier = 'premium'
WHERE name LIKE '%mistral-large%';

UPDATE models SET 
    cost_per_1k_input_tokens = 0.0003, -- Mistral Small/Medium
    cost_per_1k_output_tokens = 0.0009,
    tier = 'standard'
WHERE name LIKE '%mistral-small%' OR name LIKE '%mistral-medium%';

-- Perplexity Models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.001, -- Sonar models
    cost_per_1k_output_tokens = 0.001,
    tier = 'standard'
WHERE name LIKE '%sonar%';

-- ElevenLabs (Audio) - pricing is per character, converting to tokens
-- Assuming ~4 chars per token, so dividing by 4
-- Creator plan: $0.30 per 1K characters = $0.075 per 1K tokens
UPDATE models SET 
    cost_per_1k_tokens = 0.30, -- $0.30 per 1K characters (keeping in characters for audio)
    tier = 'standard'
WHERE name LIKE '%eleven%multilingual%' OR name LIKE '%elevenlabs%';

-- Set default tier for any unset models
UPDATE models SET tier = 'standard' WHERE tier IS NULL OR tier = '';

-- Update average latencies (estimates based on typical performance)
-- These will be updated with real data from request_logs

-- Fast models (Flash variants, small models)
UPDATE models SET avg_latency_ms = 300, avg_ttft_ms = 100, max_tps = 100
WHERE name LIKE '%flash%' OR name LIKE '%haiku%' OR name LIKE '%gpt-3.5%' OR name LIKE '%deepseek%';

-- Medium performance models
UPDATE models SET avg_latency_ms = 800, avg_ttft_ms = 300, max_tps = 50
WHERE name LIKE '%gemini-2.0%' OR name LIKE '%claude%sonnet%' OR name LIKE '%mistral%';

-- Slower, more capable models
UPDATE models SET avg_latency_ms = 2000, avg_ttft_ms = 500, max_tps = 30
WHERE name LIKE '%gpt-4%' OR name LIKE '%claude%opus%' OR name LIKE '%grok%';

-- Video/image generation models (slow)
UPDATE models SET avg_latency_ms = 15000, avg_ttft_ms = 5000, max_tps = 0
WHERE name LIKE '%veo%' OR name LIKE '%imagen%';

-- Audio models
UPDATE models SET avg_latency_ms = 1000, avg_ttft_ms = 200, max_tps = 0
WHERE name LIKE '%eleven%' OR modality = 'audio';
