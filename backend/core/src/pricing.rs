use std::collections::HashMap;

/// Pricing information for AI models
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub tier: String,
}

/// Provider-based default pricing
/// Used when specific model pricing is not available
pub struct PricingService {
    provider_defaults: HashMap<String, ModelPricing>,
    model_family_defaults: HashMap<String, ModelPricing>,
}

impl PricingService {
    pub fn new() -> Self {
        let mut provider_defaults = HashMap::new();
        let mut model_family_defaults = HashMap::new();

        // Provider-wide defaults (conservative estimates)
        provider_defaults.insert(
            "openai".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.0025,  // GPT-4o: $2.50/1M
                output_cost_per_1k: 0.010,  // GPT-4o: $10.00/1M
                tier: "premium".to_string(),
            },
        );

        provider_defaults.insert(
            "anthropic".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.003,
                output_cost_per_1k: 0.015,
                tier: "premium".to_string(),
            },
        );

        provider_defaults.insert(
            "google".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.000075, // Gemini 1.5 Flash: $0.075/1M
                output_cost_per_1k: 0.0003,  // Gemini 1.5 Flash: $0.30/1M
                tier: "standard".to_string(),
            },
        );

        provider_defaults.insert(
            "azure".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.015,  // OpenAI o1: $15.00/1M
                output_cost_per_1k: 0.060, // OpenAI o1: $60.00/1M
                tier: "premium".to_string(),
            },
        );
        
        provider_defaults.insert(
            "deepseek".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00027,  // DeepSeek V3: $0.27/1M
                output_cost_per_1k: 0.00110, // DeepSeek V3: $1.10/1M
                tier: "standard".to_string(), 
            },
        );

        provider_defaults.insert(
            "amazon".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.001,    // Nova Pro
                output_cost_per_1k: 0.004,
                tier: "standard".to_string(),
            },
        );

        provider_defaults.insert(
            "mistral".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00015, // GPT-4o mini: $0.15/1M
                output_cost_per_1k: 0.00060, // GPT-4o mini: $0.60/1M
                tier: "standard".to_string(),
            },
        );

        provider_defaults.insert(
            "perplexity".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.001,
                output_cost_per_1k: 0.001,
                tier: "standard".to_string(),
            },
        );

        provider_defaults.insert(
            "xai".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.005,
                output_cost_per_1k: 0.015,
                tier: "premium".to_string(),
            },
        );

        provider_defaults.insert(
            "elevenlabs".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.30,  // Per 1K characters
                output_cost_per_1k: 0.0,
                tier: "standard".to_string(),
            },
        );

        provider_defaults.insert(
            "selfhosted".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.0,
                output_cost_per_1k: 0.0,
                tier: "free".to_string(),
            },
        );

        provider_defaults.insert(
            "ollama".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.0,
                output_cost_per_1k: 0.0,
                tier: "free".to_string(),
            },
        );

        // Model family defaults (more specific)
        model_family_defaults.insert(
            "gpt-4".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.015,  // OpenAI o1: $15.00/1M
                output_cost_per_1k: 0.060, // OpenAI o1: $60.00/1M
                tier: "premium".to_string(),
            },
        );
        
        model_family_defaults.insert(
            "o1-mini".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.003,  // OpenAI o1-mini: $3.00/1M
                output_cost_per_1k: 0.012, // OpenAI o1-mini: $12.00/1M
                tier: "premium".to_string(),
            },
        );

        model_family_defaults.insert(
            "gpt-4o".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.005,
                output_cost_per_1k: 0.015,
                tier: "premium".to_string(),
            },
        );

        model_family_defaults.insert(
            "gpt-4o-mini".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00015, // GPT-4o mini: $0.15/1M
                output_cost_per_1k: 0.00060, // GPT-4o mini: $0.60/1M
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "gpt-3.5".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00015, // GPT-4o mini: $0.15/1M
                output_cost_per_1k: 0.00060, // GPT-4o mini: $0.60/1M
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "claude-opus".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.015,
                output_cost_per_1k: 0.075,
                tier: "premium".to_string(),
            },
        );

        model_family_defaults.insert(
            "claude-sonnet".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.003,
                output_cost_per_1k: 0.015,
                tier: "premium".to_string(),
            },
        );

        model_family_defaults.insert(
            "claude-haiku".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00080, // Claude 3.5 Haiku: $0.80/1M
                output_cost_per_1k: 0.00400, // Claude 3.5 Haiku: $4.00/1M
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "gemini-pro".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00125, // Gemini 1.5 Pro: $1.25/1M (Approx) - Vellum didn't show Pro but standard checks
                output_cost_per_1k: 0.005,
                tier: "premium".to_string(),
            },
        );

        model_family_defaults.insert(
            "gemini-flash".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.000075, // Gemini 1.5 Flash
                output_cost_per_1k: 0.0003,
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "gemini-2.0-flash".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00010, // Gemini 2.0 Flash: $0.10/1M
                output_cost_per_1k: 0.00040, // Gemini 2.0 Flash: $0.40/1M
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "deepseek-r1".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00055, // DeepSeek R1: $0.55/1M
                output_cost_per_1k: 0.00219, // DeepSeek R1: $2.19/1M
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "deepseek-v3".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00027, // DeepSeek V3
                output_cost_per_1k: 0.00110,
                tier: "standard".to_string(),
            },
        );

        model_family_defaults.insert(
            "llama-3.1-405b".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.0035, // $3.50/1M
                output_cost_per_1k: 0.0035,
                tier: "premium".to_string(),
            },
        );

        model_family_defaults.insert(
            "llama-3.3-70b".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.00059, // $0.59/1M
                output_cost_per_1k: 0.00070,
                tier: "standard".to_string(),
            },
        );

        Self {
            provider_defaults,
            model_family_defaults,
        }
    }

    /// Get pricing for a model, with fallback to provider/family defaults
    pub fn get_pricing(&self, model_name: &str, provider: &str) -> ModelPricing {
        // Try to match model family first (more specific)
        for (family, pricing) in &self.model_family_defaults {
            if model_name.to_lowercase().contains(&family.to_lowercase()) {
                return pricing.clone();
            }
        }

        // Fall back to provider default
        if let Some(pricing) = self.provider_defaults.get(&provider.to_lowercase()) {
            return pricing.clone();
        }

        // Ultimate fallback - generic pricing
        ModelPricing {
            input_cost_per_1k: 0.01,
            output_cost_per_1k: 0.03,
            tier: "standard".to_string(),
        }
    }

    /// Estimate cost for a request
    pub fn estimate_cost(
        &self,
        model_name: &str,
        provider: &str,
        input_tokens: i64,
        output_tokens: i64,
    ) -> f64 {
        let pricing = self.get_pricing(model_name, provider);
        let input_cost = (input_tokens as f64 / 1000.0) * pricing.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * pricing.output_cost_per_1k;
        input_cost + output_cost
    }

    /// Get tier classification for routing
    pub fn get_tier(&self, model_name: &str, provider: &str) -> String {
        self.get_pricing(model_name, provider).tier
    }
}

impl Default for PricingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_model_family() {
        let service = PricingService::new();
        let pricing = service.get_pricing("gpt-4-turbo-2024", "openai");
        assert_eq!(pricing.tier, "premium");
        assert!(pricing.input_cost_per_1k > 0.0);
    }

    #[test]
    fn test_provider_fallback() {
        let service = PricingService::new();
        let pricing = service.get_pricing("unknown-model", "anthropic");
        assert_eq!(pricing.tier, "premium");
        assert!(pricing.input_cost_per_1k > 0.0);
    }

    #[test]
    fn test_generic_fallback() {
        let service = PricingService::new();
        let pricing = service.get_pricing("completely-unknown", "unknown-provider");
        assert_eq!(pricing.tier, "standard");
        assert_eq!(pricing.input_cost_per_1k, 0.01);
    }

    #[test]
    fn test_cost_estimation() {
        let service = PricingService::new();
        let cost = service.estimate_cost("gpt-3.5-turbo", "openai", 1000, 500);
        // 1000 input tokens at $0.00015/1K + 500 output tokens at $0.00060/1K
        // = $0.00015 + $0.00030 = $0.00045
        assert!((cost - 0.00045).abs() < 0.0001);
    }
}
