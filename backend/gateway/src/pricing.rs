use std::collections::HashMap;

/// OpenAI model pricing per 1M tokens (as of December 2024)
/// Source: https://openai.com/api/pricing/
pub struct PricingData {
    prices: HashMap<String, ModelPricing>,
}

#[derive(Clone)]
pub struct ModelPricing {
    pub prompt_price_per_million: f64,
    pub completion_price_per_million: f64,
}

impl PricingData {
    pub fn new() -> Self {
        let mut prices = HashMap::new();

        // Note: Azure OpenAI uses the same pricing as OpenAI for the underlying models
        // Azure deployment names (e.g., "gpt-4-deployment") should map to base model names

        // GPT-4 Turbo
        prices.insert("gpt-4-turbo".to_string(), ModelPricing {
            prompt_price_per_million: 10.0,
            completion_price_per_million: 30.0,
        });
        prices.insert("gpt-4-turbo-2024-04-09".to_string(), ModelPricing {
            prompt_price_per_million: 10.0,
            completion_price_per_million: 30.0,
        });

        // GPT-4
        prices.insert("gpt-4".to_string(), ModelPricing {
            prompt_price_per_million: 30.0,
            completion_price_per_million: 60.0,
        });
        prices.insert("gpt-4-0613".to_string(), ModelPricing {
            prompt_price_per_million: 30.0,
            completion_price_per_million: 60.0,
        });

        // GPT-4o
        prices.insert("gpt-4o".to_string(), ModelPricing {
            prompt_price_per_million: 5.0,
            completion_price_per_million: 15.0,
        });
        prices.insert("gpt-4o-2024-05-13".to_string(), ModelPricing {
            prompt_price_per_million: 5.0,
            completion_price_per_million: 15.0,
        });

        // GPT-4o-mini
        prices.insert("gpt-4o-mini".to_string(), ModelPricing {
            prompt_price_per_million: 0.150,
            completion_price_per_million: 0.600,
        });
        prices.insert("gpt-4o-mini-2024-07-18".to_string(), ModelPricing {
            prompt_price_per_million: 0.150,
            completion_price_per_million: 0.600,
        });

        // GPT-3.5 Turbo
        prices.insert("gpt-3.5-turbo".to_string(), ModelPricing {
            prompt_price_per_million: 0.50,
            completion_price_per_million: 1.50,
        });
        prices.insert("gpt-3.5-turbo-0125".to_string(), ModelPricing {
            prompt_price_per_million: 0.50,
            completion_price_per_million: 1.50,
        });

        // o1 Models
        prices.insert("o1".to_string(), ModelPricing {
            prompt_price_per_million: 15.0,
            completion_price_per_million: 60.0,
        });
        prices.insert("o1-mini".to_string(), ModelPricing {
            prompt_price_per_million: 3.0,
            completion_price_per_million: 12.0,
        });

        Self { prices }
    }

    /// Calculate cost in USD for a request
    pub fn calculate_cost(&self, model: &str, prompt_tokens: i64, completion_tokens: i64) -> Option<f64> {
        self.prices.get(model).map(|pricing| {
            let prompt_cost = (prompt_tokens as f64 / 1_000_000.0) * pricing.prompt_price_per_million;
            let completion_cost = (completion_tokens as f64 / 1_000_000.0) * pricing.completion_price_per_million;
            prompt_cost + completion_cost
        })
    }

    /// Estimate cost (defaults to 0.0 if unknown)
    pub fn estimate_cost(&self, model: &str, _provider: &str, prompt_tokens: i64, completion_tokens: i64) -> f64 {
        self.calculate_cost(model, prompt_tokens, completion_tokens).unwrap_or(0.0)
    }

    /// Get pricing for a model
    pub fn get_pricing(&self, model: &str) -> Option<&ModelPricing> {
        self.prices.get(model)
    }

    // --- specialized pricing methods ---

    pub fn get_image_cost(&self, model: &str, count: i64) -> f64 {
        let base = if model.contains("dall-e-2") { 0.02 } else { 0.04 };
        base * count as f64
    }

    pub fn get_tts_cost(&self, _model: &str, chars: usize) -> f64 {
        // Standard TTS price: $0.015 per 1k characters
        (chars as f64 / 1000.0) * 0.015
    }

    pub fn get_transcription_cost(&self, _model: &str) -> f64 {
        // Fixed estimate per request for now
        0.01 
    }

    pub fn get_video_cost(&self, _model: &str) -> f64 {
        // Fixed estimate per video generation
        0.10 
    }
}

// Global pricing instance
lazy_static::lazy_static! {
    pub static ref PRICING: PricingData = PricingData::new();
}
