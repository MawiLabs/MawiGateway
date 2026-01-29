use crate::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CostManager {
    config: Arc<RwLock<AppConfig>>,
    current_spend_usd: Arc<RwLock<f64>>,
}

impl CostManager {
    pub fn new(config: Arc<RwLock<AppConfig>>) -> Self {
        Self {
            config,
            current_spend_usd: Arc::new(RwLock::new(0.0)),
        }
    }

    pub async fn check_budget(&self, estimated_cost: f64) -> Result<(), String> {
        let config = self.config.read().await;
        let spend = *self.current_spend_usd.read().await;

        if spend + estimated_cost > config.quota.daily_limit_usd {
            return Err(format!(
                "Budget exceeded: Spend=${:.4} + Est=${:.4} > Limit=${:.2}",
                spend, estimated_cost, config.quota.daily_limit_usd
            ));
        }
        Ok(())
    }

    pub async fn record_spend(&self, cost: f64) {
        let mut spend = self.current_spend_usd.write().await;
        *spend += cost;
    }
}
