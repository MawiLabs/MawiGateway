//! Agentic service types and configuration
//!
//! This module defines the primitives for agentic (agent-based) services
//! that use a planner model to orchestrate tool calls.

use serde::{Deserialize, Serialize};

/// Type of tool that an agent can call
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Enum))]
pub enum ToolType {
    /// Call another AI model directly
    Model,
    /// Call another service (which may have multiple models)
    Service,
    /// Generate an image
    ImageGeneration,
    /// Generate a video
    VideoGeneration,
    /// Text-to-speech
    TextToSpeech,
    /// Speech-to-text
    SpeechToText,
    /// Call an MCP tool (Model Context Protocol)
    Mcp,
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolType::Model => write!(f, "model"),
            ToolType::Service => write!(f, "service"),
            ToolType::ImageGeneration => write!(f, "image_generation"),
            ToolType::VideoGeneration => write!(f, "video_generation"),
            ToolType::TextToSpeech => write!(f, "text_to_speech"),
            ToolType::SpeechToText => write!(f, "speech_to_text"),
            ToolType::Mcp => write!(f, "mcp"),
        }
    }
}

impl TryFrom<String> for ToolType {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "model" => Ok(ToolType::Model),
            "service" => Ok(ToolType::Service),
            "image_generation" => Ok(ToolType::ImageGeneration),
            "video_generation" => Ok(ToolType::VideoGeneration),
            "text_to_speech" | "tts" => Ok(ToolType::TextToSpeech),
            "speech_to_text" | "stt" => Ok(ToolType::SpeechToText),
            "mcp" => Ok(ToolType::Mcp),
            _ => Err(format!("Unknown tool type: {}", s)),
        }
    }
}

/// A tool that an agentic service can invoke
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct Tool {
    /// Unique identifier for the tool
    pub id: String,
    /// Name of the tool (used in function calling)
    pub name: String,
    /// Description for the LLM to understand the tool's purpose
    pub description: String,
    /// Type of tool
    pub tool_type: ToolType,
    /// Target ID (model_id or service_name depending on tool_type)
    pub target_id: String,
    /// JSON Schema for parameters (optional, for structured tool calls)
    #[serde(default)]
    pub parameters_schema: Option<serde_json::Value>,
}

impl Tool {
    /// Convert to OpenAI function calling format
    pub fn to_openai_function(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters_schema.clone().unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "input": {
                                "type": "string",
                                "description": "The input to send to this tool"
                            }
                        },
                        "required": ["input"]
                    })
                })
            }
        })
    }
}

/// Configuration for an agentic service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct AgenticConfig {
    /// ID of the planner model (must be a chat/text model)
    pub planner_model_id: String,
    /// System prompt for the planner
    pub system_prompt: Option<String>,
    /// Available tools the agent can use
    pub tools: Vec<Tool>,
    /// Maximum number of tool-calling iterations (safety limit)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

fn default_max_iterations() -> u32 {
    10
}

impl Default for AgenticConfig {
    fn default() -> Self {
        Self {
            planner_model_id: String::new(),
            system_prompt: None,
            tools: vec![],
            max_iterations: default_max_iterations(),
        }
    }
}

/// A tool call made by the planner model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Arguments as JSON string
    pub arguments: String,
}

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct ToolResult {
    /// ID of the tool call this result is for
    pub tool_call_id: String,
    /// Result content (usually text, could be base64 for media)
    pub content: String,
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// State of an agentic execution
#[derive(Debug, Clone, Default)]
pub struct AgenticExecutionState {
    /// Current iteration number
    pub iteration: u32,
    /// Accumulated messages (conversation history)
    pub messages: Vec<AgenticMessage>,
    /// Tool calls made in this execution
    pub tool_calls_made: Vec<ToolCall>,
    /// Tool results received
    pub tool_results: Vec<ToolResult>,
    /// Whether execution is complete
    pub is_complete: bool,
    /// Final response (if complete)
    pub final_response: Option<String>,
}

/// Message in agentic conversation (supports tool calls)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl AgenticMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_to_openai_function() {
        let tool = Tool {
            id: "test-1".to_string(),
            name: "search".to_string(),
            description: "Search for information".to_string(),
            tool_type: ToolType::Service,
            target_id: "search-service".to_string(),
            parameters_schema: None,
        };

        let func = tool.to_openai_function();
        assert_eq!(func["function"]["name"], "search");
    }

    #[test]
    fn test_agentic_message_constructors() {
        let user = AgenticMessage::user("Hello");
        assert_eq!(user.role, "user");
        assert_eq!(user.content, Some("Hello".to_string()));

        let tool = AgenticMessage::tool_result("call-1", "Result");
        assert_eq!(tool.role, "tool");
        assert_eq!(tool.tool_call_id, Some("call-1".to_string()));
    }
}
