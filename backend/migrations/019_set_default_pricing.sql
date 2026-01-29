-- Set provider-based default pricing for models without explicit pricing
-- This ensures all models have at least a fallback price

-- OpenAI models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.01,
    cost_per_1k_output_tokens = 0.03,
    tier = 'premium'
WHERE provider_id LIKE '%openai%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- Anthropic models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.003,
    cost_per_1k_output_tokens = 0.015,
    tier = 'premium'
WHERE provider_id LIKE '%anthropic%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- Google models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.00125,
    cost_per_1k_output_tokens = 0.005,
    tier = 'standard'
WHERE provider_id LIKE '%google%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- Azure models without pricing  
UPDATE models SET 
    cost_per_1k_input_tokens = 0.01,
    cost_per_1k_output_tokens = 0.03,
    tier = 'premium'
WHERE provider_id LIKE '%azure%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- DeepSeek models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.0003,
    cost_per_1k_output_tokens = 0.0011,
    tier = 'free'
WHERE provider_id LIKE '%deepseek%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- Mistral models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.002,
    cost_per_1k_output_tokens = 0.006,
    tier = 'standard'
WHERE provider_id LIKE '%mistral%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- Perplexity models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.001,
    cost_per_1k_output_tokens = 0.001,
    tier = 'standard'
WHERE provider_id LIKE '%perplexity%' 
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- xAI models without pricing
UPDATE models SET 
    cost_per_1k_input_tokens = 0.005,
    cost_per_1k_output_tokens = 0.015,
    tier = 'premium'
WHERE provider_id LIKE '%xai%' OR provider_id LIKE '%x.ai%'
  AND (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0);

-- ElevenLabs models without pricing
UPDATE models SET 
    cost_per_1k_tokens = 0.30,
    tier = 'standard'
WHERE provider_id LIKE '%elevenlabs%' OR provider_id LIKE '%eleven%'
  AND (cost_per_1k_tokens IS NULL OR cost_per_1k_tokens = 0);

-- Generic fallback for any remaining models
UPDATE models SET 
    cost_per_1k_input_tokens = 0.01,
    cost_per_1k_output_tokens = 0.03,
    tier = 'standard'
WHERE (cost_per_1k_input_tokens IS NULL OR cost_per_1k_input_tokens = 0)
  AND (cost_per_1k_tokens IS NULL OR cost_per_1k_tokens = 0);

-- Set default performance metrics for models without them
UPDATE models SET 
    avg_latency_ms = 1000,
    avg_ttft_ms = 300,
    max_tps = 50
WHERE avg_latency_ms = 0 OR avg_latency_ms IS NULL;
