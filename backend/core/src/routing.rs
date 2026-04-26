use serde::{Deserialize, Serialize};

/// Routing strategies for POOL services
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
pub enum RoutingStrategy {
    /// Route to healthiest model, failover to next (default for multiple models)
    Health,
    /// Route to cheapest available model
    LeastCost,
    /// Route to fastest (lowest latency) model
    LeastLatency,
    /// Weighted random distribution based on configured weights
    WeightedRandom,
    /// No load balancing (single model or multi-modality services)
    None,
}

impl RoutingStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            RoutingStrategy::Health => "health",
            RoutingStrategy::LeastCost => "least_cost",
            RoutingStrategy::LeastLatency => "least_latency",
            RoutingStrategy::WeightedRandom => "weighted_random",
            RoutingStrategy::None => "none",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "health" => Ok(RoutingStrategy::Health),
            "least_cost" => Ok(RoutingStrategy::LeastCost),
            "least_latency" => Ok(RoutingStrategy::LeastLatency),
            "weighted_random" => Ok(RoutingStrategy::WeightedRandom),
            "none" => Ok(RoutingStrategy::None),
            _ => Err(format!("Invalid routing strategy: {}", s)),
        }
    }
}

/// Model metadata for routing decisions
#[derive(Debug, Clone)]
pub struct ModelRoutingMetadata {
    pub id: String,
    pub name: String,
    pub modality: String,

    // Health status
    pub health_status: String, // "healthy", "degraded", "unhealthy"
    pub success_rate: f64,

    // Pricing
    pub cost_per_1k_tokens: Option<f64>,
    pub tier: String,

    // Performance
    pub avg_latency_ms: i32,
    pub avg_ttft_ms: i32,

    // Assignment config
    pub weight: i32,
    pub priority: i32,
    pub enabled: bool,
}

/// Intelligent strategy recommendation engine
pub struct StrategySelector;

impl StrategySelector {
    /// Recommend optimal routing strategy based on model characteristics
    pub fn recommend_strategy(models: &[ModelRoutingMetadata], pool_type: &str) -> RoutingStrategy {
        // Single model → none
        if models.len() == 1 {
            return RoutingStrategy::None;
        }

        // Multi-modality → none (for now)
        if pool_type == "MULTI_MODALITY" {
            return RoutingStrategy::None;
        }

        // Single modality with multiple models - analyze characteristics

        // Check if all models have weights configured
        let has_weights = models.iter().all(|m| m.weight > 0);
        let total_weight: i32 = models.iter().map(|m| m.weight).sum();
        if has_weights && total_weight == 100 {
            return RoutingStrategy::WeightedRandom;
        }

        // Check cost variance
        let costs: Vec<f64> = models.iter().filter_map(|m| m.cost_per_1k_tokens).collect();

        if costs.len() >= 2 {
            let cost_variance = Self::calculate_variance(&costs);
            // If costs vary significantly (>30%), recommend cost-based routing
            if cost_variance > 0.3 {
                return RoutingStrategy::LeastCost;
            }
        }

        // Check latency variance
        let latencies: Vec<f64> = models
            .iter()
            .filter(|m| m.avg_latency_ms > 0)
            .map(|m| m.avg_latency_ms as f64)
            .collect();

        if latencies.len() >= 2 {
            let latency_variance = Self::calculate_variance(&latencies);
            // If latencies vary significantly (>30%), recommend latency-based routing
            if latency_variance > 0.3 {
                return RoutingStrategy::LeastLatency;
            }
        }

        // Default: health-based routing with failover
        RoutingStrategy::Health
    }

    /// Calculate variance (coefficient of variation) for a set of values
    fn calculate_variance(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        if mean == 0.0 {
            return 0.0;
        }

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

        let std_dev = variance.sqrt();

        // Return coefficient of variation (relative standard deviation)
        std_dev / mean
    }

    /// Validate if a strategy is compatible with the service configuration
    pub fn validate_strategy(
        strategy: &RoutingStrategy,
        models: &[ModelRoutingMetadata],
        pool_type: &str,
    ) -> Result<(), String> {
        // Single model can only use 'none' strategy
        if models.len() == 1 && *strategy != RoutingStrategy::None {
            return Err("Single model services must use 'none' strategy".to_string());
        }

        // Multi-modality can only use 'none' strategy (for now)
        if pool_type == "MULTI_MODALITY" && *strategy != RoutingStrategy::None {
            return Err("Multi-modality services must use 'none' strategy".to_string());
        }

        // Weighted strategy requires all weights to sum to 100
        if *strategy == RoutingStrategy::WeightedRandom {
            let total_weight: i32 = models.iter().map(|m| m.weight).sum();
            if total_weight != 100 {
                return Err(format!(
                    "Weighted strategy requires weights to sum to 100, got {}",
                    total_weight
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builder for the verbose `ModelRoutingMetadata` struct so the test
    /// bodies stay focused on the property under test.
    fn meta(id: &str, weight: i32, cost: Option<f64>, latency: i32) -> ModelRoutingMetadata {
        ModelRoutingMetadata {
            id: id.to_string(),
            name: id.to_string(),
            modality: "text".to_string(),
            health_status: "healthy".to_string(),
            success_rate: 0.99,
            cost_per_1k_tokens: cost,
            tier: "standard".to_string(),
            avg_latency_ms: latency,
            avg_ttft_ms: latency / 2,
            weight,
            priority: 1,
            enabled: true,
        }
    }

    // ----- recommend_strategy edge cases -----

    #[test]
    fn recommend_multi_modality_always_none() {
        let models = vec![meta("a", 50, Some(0.01), 500), meta("b", 50, Some(0.01), 500)];
        let s = StrategySelector::recommend_strategy(&models, "MULTI_MODALITY");
        assert_eq!(s, RoutingStrategy::None);
    }

    #[test]
    fn recommend_uniform_costs_falls_through_to_health() {
        // Equal cost + equal latency + weights not summing to 100
        // → no variance signal, default to Health.
        let models = vec![
            meta("a", 40, Some(0.01), 500),
            meta("b", 40, Some(0.01), 500),
        ];
        let s = StrategySelector::recommend_strategy(&models, "SINGLE_MODALITY");
        assert_eq!(s, RoutingStrategy::Health);
    }

    #[test]
    fn recommend_latency_variance_picks_least_latency() {
        // Costs equal, latencies wildly different → least_latency wins.
        let models = vec![
            meta("fast", 40, Some(0.01), 100),
            meta("slow", 40, Some(0.01), 5_000),
        ];
        let s = StrategySelector::recommend_strategy(&models, "SINGLE_MODALITY");
        assert_eq!(s, RoutingStrategy::LeastLatency);
    }

    // ----- validate_strategy contract -----

    #[test]
    fn validate_single_model_must_use_none() {
        let models = vec![meta("only", 100, None, 500)];
        for s in [
            RoutingStrategy::Health,
            RoutingStrategy::LeastCost,
            RoutingStrategy::LeastLatency,
            RoutingStrategy::WeightedRandom,
        ] {
            let err = StrategySelector::validate_strategy(&s, &models, "SINGLE_MODALITY")
                .expect_err("single-model should reject non-None strategy");
            assert!(err.contains("Single model"), "got: {}", err);
        }
        // None is the only acceptable choice.
        StrategySelector::validate_strategy(&RoutingStrategy::None, &models, "SINGLE_MODALITY")
            .unwrap();
    }

    #[test]
    fn validate_multi_modality_must_use_none() {
        let models = vec![meta("a", 50, None, 500), meta("b", 50, None, 500)];
        let err = StrategySelector::validate_strategy(
            &RoutingStrategy::WeightedRandom,
            &models,
            "MULTI_MODALITY",
        )
        .expect_err("multi-modality must use None");
        assert!(err.contains("Multi-modality"), "got: {}", err);
    }

    #[test]
    fn validate_weighted_strategy_requires_weights_summing_to_100() {
        let models = vec![meta("a", 70, None, 500), meta("b", 20, None, 500)]; // 90
        let err = StrategySelector::validate_strategy(
            &RoutingStrategy::WeightedRandom,
            &models,
            "SINGLE_MODALITY",
        )
        .expect_err("weights must sum to 100");
        assert!(err.contains("100"), "got: {}", err);
        assert!(err.contains("90"), "should mention actual sum, got: {}", err);

        // Sum = 100 → ok.
        let ok_models = vec![meta("a", 70, None, 500), meta("b", 30, None, 500)];
        StrategySelector::validate_strategy(
            &RoutingStrategy::WeightedRandom,
            &ok_models,
            "SINGLE_MODALITY",
        )
        .unwrap();
    }

    #[test]
    fn validate_non_weighted_strategies_ignore_weight_sum() {
        // Health / LeastCost / LeastLatency don't care that weights
        // don't sum to 100 — they have their own selection logic.
        let models = vec![meta("a", 70, Some(0.01), 500), meta("b", 20, Some(0.02), 600)];
        for s in [
            RoutingStrategy::Health,
            RoutingStrategy::LeastCost,
            RoutingStrategy::LeastLatency,
        ] {
            StrategySelector::validate_strategy(&s, &models, "SINGLE_MODALITY")
                .unwrap_or_else(|e| panic!("strategy {:?} should pass: {}", s, e));
        }
    }

    // ----- calculate_variance edge cases -----

    #[test]
    fn variance_of_empty_list_is_zero() {
        assert_eq!(StrategySelector::calculate_variance(&[]), 0.0);
    }

    #[test]
    fn variance_of_zero_mean_is_zero() {
        assert_eq!(StrategySelector::calculate_variance(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn variance_of_uniform_values_is_zero() {
        assert_eq!(StrategySelector::calculate_variance(&[5.0, 5.0, 5.0]), 0.0);
    }

    #[test]
    fn variance_increases_with_spread() {
        let tight = StrategySelector::calculate_variance(&[10.0, 11.0, 9.0]);
        let wide = StrategySelector::calculate_variance(&[1.0, 50.0, 100.0]);
        assert!(wide > tight, "wider spread should yield larger variance");
    }

    #[test]
    fn test_single_model_strategy() {
        let models = vec![ModelRoutingMetadata {
            id: "model1".to_string(),
            name: "GPT-4".to_string(),
            modality: "text".to_string(),
            health_status: "healthy".to_string(),
            success_rate: 0.99,
            cost_per_1k_tokens: Some(0.03),
            tier: "premium".to_string(),
            avg_latency_ms: 500,
            avg_ttft_ms: 200,
            weight: 100,
            priority: 1,
            enabled: true,
        }];

        let strategy = StrategySelector::recommend_strategy(&models, "SINGLE_MODALITY");
        assert_eq!(strategy, RoutingStrategy::None);
    }

    #[test]
    fn test_cost_variance_strategy() {
        let models = vec![
            ModelRoutingMetadata {
                id: "model1".to_string(),
                name: "GPT-4".to_string(),
                modality: "text".to_string(),
                health_status: "healthy".to_string(),
                success_rate: 0.99,
                cost_per_1k_tokens: Some(0.03), // Expensive
                tier: "premium".to_string(),
                avg_latency_ms: 500,
                avg_ttft_ms: 200,
                weight: 50,
                priority: 1,
                enabled: true,
            },
            ModelRoutingMetadata {
                id: "model2".to_string(),
                name: "GPT-3.5".to_string(),
                modality: "text".to_string(),
                health_status: "healthy".to_string(),
                success_rate: 0.98,
                cost_per_1k_tokens: Some(0.001), // Much cheaper
                tier: "standard".to_string(),
                avg_latency_ms: 300,
                avg_ttft_ms: 100,
                weight: 40, // Sum = 90 < 100, so falls through to cost variance
                priority: 2,
                enabled: true,
            },
        ];

        let strategy = StrategySelector::recommend_strategy(&models, "SINGLE_MODALITY");
        assert_eq!(strategy, RoutingStrategy::LeastCost);
    }

    #[test]
    fn test_weighted_strategy_with_proper_weights() {
        let models = vec![
            ModelRoutingMetadata {
                id: "model1".to_string(),
                name: "Model 1".to_string(),
                modality: "text".to_string(),
                health_status: "healthy".to_string(),
                success_rate: 0.99,
                cost_per_1k_tokens: Some(0.01),
                tier: "standard".to_string(),
                avg_latency_ms: 500,
                avg_ttft_ms: 200,
                weight: 70,
                priority: 1,
                enabled: true,
            },
            ModelRoutingMetadata {
                id: "model2".to_string(),
                name: "Model 2".to_string(),
                modality: "text".to_string(),
                health_status: "healthy".to_string(),
                success_rate: 0.98,
                cost_per_1k_tokens: Some(0.01),
                tier: "standard".to_string(),
                avg_latency_ms: 500,
                avg_ttft_ms: 200,
                weight: 30,
                priority: 2,
                enabled: true,
            },
        ];

        let strategy = StrategySelector::recommend_strategy(&models, "SINGLE_MODALITY");
        assert_eq!(strategy, RoutingStrategy::WeightedRandom);
    }
}
