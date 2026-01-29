use serde::{Deserialize, Serialize};
use crate::routing::RoutingStrategy;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ChatParams {
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UnifiedChatRequest {
    pub service: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub params: Option<ChatParams>,
    #[serde(default)]
    pub stream: Option<bool>,
    
    // Optional Override to force specific model within service context
    #[serde(default)]
    pub model: Option<String>,
    
    // NEW: Optional routing strategy override
    #[serde(default)]
    pub routing_strategy: Option<RoutingStrategy>,
    
    // NEW: Response format (JSON Mode)
    #[serde(default)]
    pub response_format: Option<crate::types::ResponseFormat>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct UnifiedChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<TokenUsage>,
    pub routing_metadata: Option<RoutingMetadata>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ChatChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct RoutingMetadata {
    pub requested_routing: RequestedRouting,
    pub actual_routing: ActualRouting,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct RequestedRouting {
    pub service: String,
    pub model_override: Option<String>,
    pub routing_strategy: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ActualRouting {
    pub provider: String,
    pub model: String,
    pub fallback_used: bool,
}

/// Events emitted during agentic execution stream
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum AgenticStreamEvent {
    /// A generic log message (e.g. status update)
    #[serde(rename = "log")]
    Log { step: String, content: String },       
    
    /// Tool execution started
    #[serde(rename = "tool_start")]
    ToolStart { tool: String, input: String },   
    
    /// Tool execution finished
    #[serde(rename = "tool_end")]
    ToolEnd { tool: String, output: String },    
    
    /// High-level step started (Plan step)
    #[serde(rename = "step_start")]
    StepStart { step_number: i32, desc: String },
    
    /// Final answer chunk (standard text delta)
    #[serde(rename = "chunk")]
    FinalResponse(String),
    
    /// Incremental token for reasoning (thought process)
    #[serde(rename = "reasoning_delta")]
    ReasoningDelta(String),
}
