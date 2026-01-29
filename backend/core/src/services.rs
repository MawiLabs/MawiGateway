use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, postgres::PgRow};

// Service type enums
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
pub enum ServiceType {
    Pool,
    Agentic,
}

impl TryFrom<String> for ServiceType {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_uppercase().as_str() {
            "POOL" => Ok(ServiceType::Pool),
            "AGENTIC" => Ok(ServiceType::Agentic),
            "CHAT" | "AUDIO" | "VIDEO" => Ok(ServiceType::Pool), // Legacy values default to Pool
            _ => Err(format!("Unknown service type: {}", s)),
        }
    }
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Pool => write!(f, "POOL"),
            ServiceType::Agentic => write!(f, "AGENTIC"),
        }
    }
}

impl ServiceType {
    pub fn as_str(&self) -> &str {
        match self {
            ServiceType::Pool => "POOL",
            ServiceType::Agentic => "AGENTIC",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
pub enum PoolType {
    SingleModality,
    MultiModality,
}

impl TryFrom<String> for PoolType {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_uppercase().as_str() {
            "SINGLE_MODALITY" => Ok(PoolType::SingleModality),
            "MULTI_MODALITY" => Ok(PoolType::MultiModality),
            _ => Err(format!("Unknown pool type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct Service {
    pub name: String, // Primary key
    pub service_type: ServiceType,
    pub description: Option<String>,
    pub strategy: String, // For backward compat
    pub guardrails: Option<String>, // JSON array stored as string
    pub created_at: Option<i64>,
    
    // NEW: Pool-specific fields
    pub pool_type: Option<PoolType>,
    pub input_modalities: Vec<Modality>,
    pub output_modalities: Vec<Modality>,
    
    // NEW: Agentic-specific fields
    pub planner_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub max_iterations: Option<i32>,
    
    // Ownership
    pub user_id: Option<String>,
}

impl<'r> FromRow<'r, PgRow> for Service {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let name: String = row.try_get("name")?;
        
        // Parse service_type with backward compat
        let service_type_str: String = row.try_get("service_type")?;
        let service_type = ServiceType::try_from(service_type_str)
            .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?;
        
        // Parse pool_type (optional)
        let pool_type: Option<String> = row.try_get("pool_type").ok();
        let pool_type = pool_type.and_then(|s| PoolType::try_from(s).ok());
        
        // Parse JSON modalities
        let input_modalities_str: String = row.try_get("input_modalities")
            .unwrap_or_else(|_| "[\"text\"]".to_string());
        let input_modalities: Vec<Modality> = serde_json::from_str(&input_modalities_str)
            .unwrap_or_else(|_| vec![Modality::Text]);
        
        let output_modalities_str: String = row.try_get("output_modalities")
            .unwrap_or_else(|_| "[\"text\"]".to_string());
        let output_modalities: Vec<Modality> = serde_json::from_str(&output_modalities_str)
            .unwrap_or_else(|_| vec![Modality::Text]);
        
        Ok(Service {
            name,
            service_type,
            description: row.try_get("description").ok(),
            strategy: row.try_get("strategy").unwrap_or_else(|_| "weighted".to_string()),
            guardrails: row.try_get("guardrails").ok(),
            created_at: row.try_get("created_at").ok(),
            pool_type,
            input_modalities,
            output_modalities,
            planner_model_id: row.try_get("planner_model_id").ok(),
            system_prompt: row.try_get("system_prompt").ok(),
            max_iterations: row.try_get("max_iterations").ok(),
            user_id: row.try_get("user_id").ok(),
        })
    }
}


#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct CreateService {
    pub name: String,
    pub service_type: String, // 'POOL' or 'AGENTIC'
    pub description: Option<String>,
    pub strategy: Option<String>,
    #[serde(default)]
    pub guardrails: Vec<String>, // Array of guardrail IDs (default: empty)
    
    // Agentic-specific fields
    pub planner_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UpdateService {
    pub service_type: Option<String>,
    pub description: Option<String>,
    pub strategy: Option<String>,
    pub guardrails: Option<Vec<String>>,
    
    // Pool-specific
    pub pool_type: Option<String>,

    // Agentic-specific fields
    pub planner_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct AssignModel {
    pub model_id: String,
    pub modality: String,
    pub position: i32,
    #[serde(default = "default_weight")]
    pub weight: i32,  // For weighted distribution
    
    // RTCROS advanced settings (System Prompt Components)
    pub rtcros_role: Option<String>,
    pub rtcros_task: Option<String>,
    pub rtcros_context: Option<String>,
    pub rtcros_reasoning: Option<String>,
    pub rtcros_output: Option<String>,
    pub rtcros_stop: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UpdateModelAssignment {
    pub position: Option<i32>,
    pub weight: Option<i32>,
    pub rtcros_role: Option<String>,
    pub rtcros_task: Option<String>,
    pub rtcros_context: Option<String>,
    pub rtcros_reasoning: Option<String>,
    pub rtcros_output: Option<String>,
    pub rtcros_stop: Option<String>,
}

fn default_weight() -> i32 {
    100
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct BulkUpdateModelAssignment {
    pub model_id: String,
    pub position: Option<i32>,
    pub weight: Option<i32>,
    pub rtcros_role: Option<String>,
    pub rtcros_task: Option<String>,
    pub rtcros_context: Option<String>,
    pub rtcros_reasoning: Option<String>,
    pub rtcros_output: Option<String>,
    pub rtcros_stop: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct BulkUpdateServiceModels {
    pub models: Vec<BulkUpdateModelAssignment>,
}
