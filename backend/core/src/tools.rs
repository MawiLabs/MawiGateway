use serde::{Deserialize, Serialize};

/// Request to create a tool for an agentic service
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct CreateTool {
    pub name: String,
    pub description: String,
    pub tool_type: String, // 'model', 'service', 'image_generation', etc.
    pub target_id: String,  // model_id or service_name
    pub parameters_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub position: i32,
}

/// Request to update a tool
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UpdateTool {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tool_type: Option<String>,
    pub target_id: Option<String>,
    pub parameters_schema: Option<serde_json::Value>,
    pub position: Option<i32>,
}
