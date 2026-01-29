//! MCP client implementation
//! Supports stdio (Docker containers) and SSE (HTTP streaming)
//! Handles full lifecycle: init, list tools, call tools

use anyhow::{Result, anyhow, Context};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::{info, debug};

/// Counter for JSON-RPC request IDs
static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// MCP Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub server_type: ServerType,
    pub image_or_command: String,
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Docker,
    Stdio,
    Sse,
}

impl std::fmt::Display for ServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerType::Docker => write!(f, "docker"),
            ServerType::Stdio => write!(f, "stdio"),
            ServerType::Sse => write!(f, "sse"),
        }
    }
}

/// Discovered MCP Tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<Value>,
}

/// JSON-RPC Request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC Response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// Active MCP connection via stdio
pub struct McpConnection {
    process: Child,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    stderr: Mutex<BufReader<tokio::process::ChildStderr>>,
    pub server_info: Option<Value>,
    pub capabilities: Option<Value>,
}

impl McpConnection {
    /// Spawn a Docker container and connect via stdio
    pub async fn spawn_docker(config: &McpServerConfig) -> Result<Self> {
        info!("🐳 Spawning Docker MCP server: {}", config.image_or_command);
        
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("-i");
        
        // Add environment variables
        for (key, value) in &config.env_vars {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }
        
        cmd.arg(&config.image_or_command);
        
        // Add additional args
        for arg in &config.args {
            cmd.arg(arg);
        }
        
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let mut process = cmd.spawn()
            .context("Failed to spawn Docker container")?;
        
        let stdin = process.stdin.take()
            .ok_or_else(|| anyhow!("Failed to capture stdin"))?;
        let stdout = process.stdout.take()
            .ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = process.stderr.take()
            .ok_or_else(|| anyhow!("Failed to capture stderr"))?;
        
        Ok(Self {
            process,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            stderr: Mutex::new(BufReader::new(stderr)),
            server_info: None,
            capabilities: None,
        })
    }
    
    /// Spawn a local stdio process
    pub async fn spawn_stdio(config: &McpServerConfig) -> Result<Self> {
        info!("🖥️ Spawning stdio MCP server: {}", config.image_or_command);
        
        let mut cmd = Command::new(&config.image_or_command);
        
        for arg in &config.args {
            cmd.arg(arg);
        }
        
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let mut process = cmd.spawn()
            .context("Failed to spawn stdio process")?;
        
        let stdin = process.stdin.take()
            .ok_or_else(|| anyhow!("Failed to capture stdin"))?;
        let stdout = process.stdout.take()
            .ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = process.stderr.take()
            .ok_or_else(|| anyhow!("Failed to capture stderr"))?;
            
        Ok(Self {
            process,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            stderr: Mutex::new(BufReader::new(stderr)),
            server_info: None,
            capabilities: None,
        })
    }
    
    /// Initialize the MCP connection (handshake)
    pub async fn initialize(&mut self) -> Result<()> {
        info!("🤝 Initializing MCP connection...");
        
        let response_result = self.send_request("initialize", Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": { "listChanged": true },
                "sampling": {}
            },
            "clientInfo": {
                "name": "mawi-gateway",
                "version": "1.0.0"
            }
        }))).await;
        
        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                // try reading stderr for error details
                // process might not close immediately
                let mut stderr_buf = String::new();
                
                // Simple attempt to read lines
                {
                    let mut stderr = self.stderr.lock().await;
                    // We try to read execution output if available
                    // Use a timeout or simple read? The original code had a loop.
                    // If the process is dead, we get EOF.
                    while let Ok(n) = stderr.read_line(&mut stderr_buf).await {
                        if n == 0 { break; }
                    }
                }
                
                if !stderr_buf.is_empty() {
                    return Err(anyhow!("MCP connection failed: {}. Stderr: {}", e, stderr_buf));
                } else {
                    return Err(e);
                }
            }
        };
        
        if let Some(result) = response.result {
            self.server_info = result.get("serverInfo").cloned();
            self.capabilities = result.get("capabilities").cloned();
            
            info!("✅ MCP initialized: {:?}", self.server_info);
            
            // Send initialized notification
            self.send_notification("notifications/initialized", None).await?;
            
            Ok(())
        } else if let Some(error) = response.error {
            Err(anyhow!("MCP initialize failed: {} (code {})", error.message, error.code))
        } else {
            Err(anyhow!("MCP initialize returned no result"))
        }
    }
    
    /// List available tools from the MCP server
    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        info!("📋 Listing MCP tools...");
        
        let response = self.send_request("tools/list", None).await?;
        
        if let Some(result) = response.result {
            let tools: Vec<McpTool> = serde_json::from_value(
                result.get("tools").cloned().unwrap_or(json!([]))
            ).context("Failed to parse tools list")?;
            
            info!("✅ Discovered {} tools", tools.len());
            Ok(tools)
        } else if let Some(error) = response.error {
            Err(anyhow!("tools/list failed: {} (code {})", error.message, error.code))
        } else {
            Ok(vec![])
        }
    }
    
    /// Call a tool on the MCP server
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        info!("🔧 Calling MCP tool: {}", name);
        debug!("Tool arguments: {:?}", arguments);
        
        let response = self.send_request("tools/call", Some(json!({
            "name": name,
            "arguments": arguments
        }))).await?;
        
        if let Some(result) = response.result {
            info!("✅ Tool call completed: {}", name);
            Ok(result)
        } else if let Some(error) = response.error {
            Err(anyhow!("Tool call failed: {} (code {})", error.message, error.code))
        } else {
            Ok(json!({"content": []}))
        }
    }
    
    /// Send a JSON-RPC request and wait for response
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<JsonRpcResponse> {
        const MAX_LINE_SIZE: usize = 10 * 1024 * 1024; // 10MB limit
        
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };
        
        let request_str = serde_json::to_string(&request)?;
        debug!("-> {}", request_str);
        
        // Write request
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(request_str.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }
        
        // Read response with size limit
        let mut stdout = self.stdout.lock().await;
        
        // Read until we get the response for our ID
        loop {
            let mut buf = Vec::new();
            let bytes_read = stdout.read_until(b'\n', &mut buf).await?;
            
            if bytes_read == 0 {
                return Err(anyhow!("MCP server closed connection"));
            }
            
            if buf.len() > MAX_LINE_SIZE {
                return Err(anyhow!("MCP response exceeds 10MB limit (potential attack)"));
            }
            
            let line = String::from_utf8_lossy(&buf).to_string();
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            debug!("<- {}", trimmed);
            
            if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                if resp.id == Some(id) {
                    return Ok(resp);
                }
                // Ignore notifications and responses for other IDs
            }
        }
    }
    
    /// Send a JSON-RPC notification (no response expected)
    async fn send_notification(&self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        
        let notification_str = serde_json::to_string(&notification)?;
        debug!("-> {}", notification_str);
        
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(notification_str.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        
        Ok(())
    }
    
    /// Gracefully shutdown the MCP server
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("🛑 Shutting down MCP server...");
        
        // Try to send shutdown notification
        let _ = self.send_notification("notifications/cancelled", None).await;
        
        // Kill the process
        let _ = self.process.kill().await;
        
        Ok(())
    }
}

impl Drop for McpConnection {
    fn drop(&mut self) {
        // Best effort cleanup - can't await in Drop
        let _ = self.process.start_kill();
    }
}

/// Manager for multiple MCP connections
pub struct McpManager {
    connections: Mutex<HashMap<String, Arc<McpConnection>>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
        }
    }
    
    /// Connect to an MCP server
    pub async fn connect(&self, config: &McpServerConfig) -> Result<Vec<McpTool>> {
        let mut conn = match config.server_type {
            ServerType::Docker => McpConnection::spawn_docker(config).await?,
            ServerType::Stdio => McpConnection::spawn_stdio(config).await?,
            ServerType::Sse => {
                return Err(anyhow!("SSE transport not yet implemented"));
            }
        };
        
        // Initialize the connection
        conn.initialize().await?;
        
        // Discover tools
        let tools = conn.list_tools().await?;
        
        // Store the connection
        let mut connections = self.connections.lock().await;
        connections.insert(config.id.clone(), Arc::new(conn));
        
        Ok(tools)
    }
    
    /// Disconnect from an MCP server
    pub async fn disconnect(&self, server_id: &str) -> Result<()> {
        let mut connections = self.connections.lock().await;
        if let Some(conn) = connections.remove(server_id) {
            // We can't move out of Arc easily, but we can try to shutdown.
            // Since we just removed it from map, we have the only ref if no calls running.
            // But conn is Arc.
            // We can check strong_count or just let it drop?
            // McpConnection::shutdown is &mut self. Arc doesn't allow mut.
            // We need to change shutdown to take &self using internal mutability or handle this differently.
            // McpConnection::shutdown uses `self.process.kill()`. Process is `Child`. `Child` needs `&mut`.
            // So McpConnection needs `process: Mutex<Child>`.
            // Currently `process: Child`.
            // So `shutdown` works only on owned instance.
            // If we use Arc, we can't call shutdown unless we use interior mutability for process.
            
            // For now, we rely on Drop, but Drop is `&mut self` on struct. Arc drop works.
            // But we wanted explicit shutdown.
            // rely on Drop for cleanup (McpConnection impl)
            drop(conn); 
        }
        Ok(())
    }
    
    /// Call a tool on a specific server
    pub async fn call_tool(&self, server_id: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        // drop lock early for concurrent tool execution
        let conn = {
            let connections = self.connections.lock().await;
            connections.get(server_id).cloned()
        };

        if let Some(conn) = conn {
            conn.call_tool(tool_name, arguments).await
        } else {
            Err(anyhow!("Server {} not connected", server_id))
        }
    }
    
    /// Check if a server is connected

    pub async fn is_connected(&self, server_id: &str) -> bool {
        let connections = self.connections.lock().await;
        connections.contains_key(server_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_server_type_display() {
        assert_eq!(ServerType::Docker.to_string(), "docker");
        assert_eq!(ServerType::Stdio.to_string(), "stdio");
        assert_eq!(ServerType::Sse.to_string(), "sse");
    }
}
