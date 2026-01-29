use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub monthly_quota_usd: f64,
    pub is_free: bool,
    pub max_users: Option<i32>, // None = unlimited (or defined by org limits)
    pub features: Vec<String>,
}

pub const PLAN_FREE_TIER_ID: &str = "community";
pub const PLAN_PRO_TIER_ID: &str = "team";
pub const PLAN_ENTERPRISE_ID: &str = "enterprise";

pub fn get_plans() -> Vec<Plan> {
    vec![
        Plan {
            id: PLAN_FREE_TIER_ID.to_string(),
            name: "Community".to_string(),
            monthly_quota_usd: 5.0,
            is_free: true,
            max_users: Some(1), // Individual use primarily
            features: vec![
                "community_support".to_string(),
                "basic_models".to_string(),
                "rate_limited".to_string(),
            ],
        },
        Plan {
            id: PLAN_PRO_TIER_ID.to_string(),
            name: "Team".to_string(),
            monthly_quota_usd: 50.0, // Example
            is_free: false,
            max_users: Some(5),
            features: vec![
                "email_support".to_string(),
                "advanced_models".to_string(),
                "increased_limits".to_string(),
            ],
        },
         Plan {
            id: PLAN_ENTERPRISE_ID.to_string(),
            name: "Enterprise".to_string(),
            monthly_quota_usd: 1000.0, // Custom
            is_free: false,
            max_users: None,
            features: vec![
                "sso".to_string(),
                "audit_logs".to_string(),
                "dedicated_support".to_string(),
            ],
        },
    ]
}

pub fn get_default_plan() -> Plan {
    get_plans()
        .into_iter()
        .find(|p| p.id == PLAN_FREE_TIER_ID)
        .expect("Free tier must exist")
}

pub fn get_plan_by_id(id: &str) -> Option<Plan> {
    get_plans().into_iter().find(|p| p.id == id)
}
