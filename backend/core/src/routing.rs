use serde::{Deserialize, Serialize};

/// Routing strategies for POOL services.
///
/// Strategy strings on `services.strategy` rows are parsed via [`parse`][Self::parse],
/// which accepts canonical names (the values returned by [`as_str`][Self::as_str])
/// plus legacy aliases — single source of truth for the executor's dispatch.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Even distribution by request count (per-service in-memory counter)
    RoundRobin,
    /// No load balancing (single model or multi-modality services)
    None,
}

impl RoutingStrategy {
    /// Canonical wire name. Stable; legacy aliases map to these via [`parse`][Self::parse].
    pub fn as_str(&self) -> &str {
        match self {
            RoutingStrategy::Health => "health",
            RoutingStrategy::LeastCost => "least_cost",
            RoutingStrategy::LeastLatency => "least_latency",
            RoutingStrategy::WeightedRandom => "weighted_random",
            RoutingStrategy::RoundRobin => "round_robin",
            RoutingStrategy::None => "none",
        }
    }

    /// Parse a strategy string from service config, accepting legacy aliases.
    /// Returns `None` for unrecognised input — callers typically log a warning
    /// and fall back to a sensible default.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            // Health / priority failover — "try in order, failover on error"
            "health" | "leader-worker" | "leader_worker" | "priority"
            | "highest_quality" | "failover" => Some(RoutingStrategy::Health),

            // Weighted random distribution
            "weighted_random" | "weighted" | "random" | "pool" => {
                Some(RoutingStrategy::WeightedRandom)
            }

            // Cost-optimised
            "least_cost" | "cheapest" | "cost" => Some(RoutingStrategy::LeastCost),

            // Latency-optimised
            "least_latency" | "speed" | "fastest" | "latency" => {
                Some(RoutingStrategy::LeastLatency)
            }

            // Round-robin
            "round_robin" | "round-robin" | "rr" => Some(RoutingStrategy::RoundRobin),

            // Single model / no balancing
            "none" | "" => Some(RoutingStrategy::None),

            _ => None,
        }
    }

    /// Strict parse: errors on unknown strategy. Use in API validation
    /// paths where the caller should be told they typoed; for runtime
    /// dispatch where we want a fallback, use [`parse`][Self::parse].
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        Self::parse(s).ok_or_else(|| format!("Invalid routing strategy: {}", s))
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
    fn parse_canonical_names() {
        assert_eq!(RoutingStrategy::parse("health"), Some(RoutingStrategy::Health));
        assert_eq!(RoutingStrategy::parse("least_cost"), Some(RoutingStrategy::LeastCost));
        assert_eq!(RoutingStrategy::parse("least_latency"), Some(RoutingStrategy::LeastLatency));
        assert_eq!(
            RoutingStrategy::parse("weighted_random"),
            Some(RoutingStrategy::WeightedRandom)
        );
        assert_eq!(RoutingStrategy::parse("round_robin"), Some(RoutingStrategy::RoundRobin));
        assert_eq!(RoutingStrategy::parse("none"), Some(RoutingStrategy::None));
    }

    #[test]
    fn parse_legacy_aliases() {
        // Health aliases — strings that operators have on existing services rows.
        for alias in ["leader-worker", "leader_worker", "priority", "highest_quality", "failover"] {
            assert_eq!(
                RoutingStrategy::parse(alias),
                Some(RoutingStrategy::Health),
                "alias {} did not map to Health",
                alias
            );
        }
        // Weighted aliases.
        for alias in ["weighted", "random", "pool"] {
            assert_eq!(RoutingStrategy::parse(alias), Some(RoutingStrategy::WeightedRandom));
        }
        // Latency aliases — "speed" was in the executor's match arms.
        for alias in ["speed", "fastest", "latency"] {
            assert_eq!(RoutingStrategy::parse(alias), Some(RoutingStrategy::LeastLatency));
        }
    }

    #[test]
    fn parse_normalizes_case_and_whitespace() {
        assert_eq!(RoutingStrategy::parse("  HEALTH  "), Some(RoutingStrategy::Health));
        assert_eq!(RoutingStrategy::parse("Round-Robin"), Some(RoutingStrategy::RoundRobin));
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert_eq!(RoutingStrategy::parse("weighted_radom"), None); // common typo
        assert_eq!(RoutingStrategy::parse("magic"), None);
    }

    #[test]
    fn from_str_errors_on_unknown() {
        assert!(RoutingStrategy::from_str("not_a_strategy").is_err());
        assert!(RoutingStrategy::from_str("health").is_ok());
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
