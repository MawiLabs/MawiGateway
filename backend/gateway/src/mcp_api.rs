//! MCP Servers API
//!
//! REST API for managing MCP server connections, discovering tools,
//! and executing MCP tool calls.

use poem::Result;
use poem_openapi::{param::Path, param::Query, payload::Json, Object, OpenApi, Tags};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::mcp_client::{McpManager, McpServerConfig, ServerType};

#[derive(Tags)]
enum ApiTags {
    /// MCP Server Management
    Mcp,
}

/// MCP Server stored in database
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub image_or_command: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,
}

/// Request to create a new MCP server
#[derive(Debug, Deserialize, Object)]
pub struct CreateMcpServerRequest {
    pub name: String,
    pub server_type: String,
    pub image_or_command: String,
    #[serde(default)]
    #[oai(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    #[oai(default)]
    pub env_vars: Option<HashMap<String, String>>,
}

/// Request to update an MCP server
#[derive(Debug, Deserialize, Object)]
pub struct UpdateMcpServerRequest {
    pub name: Option<String>,
    pub server_type: Option<String>,
    pub image_or_command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env_vars: Option<HashMap<String, String>>,
}

/// Discovered tool from MCP server
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct McpToolResponse {
    pub id: String,
    pub server_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Response for server connection
#[derive(Debug, Serialize, Object)]
pub struct ConnectResponse {
    pub status: String,
    pub tools_discovered: usize,
}

pub struct McpApi {
    pub pool: PgPool,
    manager: Arc<RwLock<McpManager>>,
}

impl McpApi {
    pub fn new(pool: PgPool, manager: Arc<RwLock<McpManager>>) -> Self {
        Self { pool, manager }
    }
}

#[OpenApi]
impl McpApi {
    /// List all MCP servers — multi-tenant: scoped to the authenticated
    /// user. Pre-multi-tenant rows with NULL owner remain visible.
    #[oai(path = "/mcp/servers", method = "get", tag = "ApiTags::Mcp")]
    async fn list_servers(
        &self,
        Query(limit): Query<Option<i64>>,
        Query(offset): Query<Option<i64>>,
        poem_req: &poem::Request,
    ) -> Result<Json<Vec<McpServer>>> {
        let user_id = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;

        let page = crate::pagination::Pagination::from_parts(limit, offset);
        let rows = sqlx::query(
            r#"SELECT id, name, server_type, image_or_command, status, error_message,
                      created_at, args, env_vars
               FROM mcp_servers
               WHERE user_id = $1
               ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(&user_id)
        .bind(page.limit)
        .bind(page.offset)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let servers: Vec<McpServer> = rows
            .into_iter()
            .map(|row| {
                let args_str: Option<String> = row.get("args");
                let env_str: Option<String> = row.get("env_vars");

                McpServer {
                    id: row.get("id"),
                    name: row.get("name"),
                    server_type: row.get("server_type"),
                    image_or_command: row.get("image_or_command"),
                    status: row.get("status"),
                    error_message: row.get("error_message"),
                    created_at: row.get("created_at"),
                    args: args_str.and_then(|s| serde_json::from_str(&s).ok()),
                    env_vars: env_str.and_then(|s| serde_json::from_str(&s).ok()),
                }
            })
            .collect();

        Ok(Json(servers))
    }

    /// Create a new MCP server configuration
    #[oai(path = "/mcp/servers", method = "post", tag = "ApiTags::Mcp")]
    async fn create_server(
        &self,
        body: Json<CreateMcpServerRequest>,
        poem_req: &poem::Request,
    ) -> Result<Json<McpServer>> {
        // Get the authenticated user from the request (injected by AuthMiddleware)
        let user = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;
        let user_id = &user.id;

        let id = Uuid::new_v4().to_string();
        let args_json =
            serde_json::to_string(&body.args.clone().unwrap_or_default()).unwrap_or_default();
        let env_json =
            serde_json::to_string(&body.env_vars.clone().unwrap_or_default()).unwrap_or_default();
        let now = chrono::Utc::now().timestamp();

        eprintln!(
            "Creating MCP server: name={}, type={}, image={}, user={}",
            body.name, body.server_type, body.image_or_command, user_id
        );

        sqlx::query(
            r#"INSERT INTO mcp_servers (id, user_id, name, server_type, image_or_command, args, env_vars, status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'disconnected', $8, $9)"#
        )
        .bind(&id)
        .bind(user_id)
        .bind(&body.name)
        .bind(&body.server_type)
        .bind(&body.image_or_command)
        .bind(&args_json)
        .bind(&env_json)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Failed to create MCP server: {}", e);
            poem::Error::from_string(
                format!("Failed to create server: {}", e),
                poem::http::StatusCode::INTERNAL_SERVER_ERROR
            )
        })?;

        eprintln!("Created MCP server: {} ({})", body.name, id);

        Ok(Json(McpServer {
            id,
            name: body.name.clone(),
            server_type: body.server_type.clone(),
            image_or_command: body.image_or_command.clone(),
            status: "disconnected".to_string(),
            error_message: None,
            created_at: now,
            args: body.args.clone(),
            env_vars: body.env_vars.clone(),
        }))
    }

    /// Update an MCP server configuration. Multi-tenant: caller must own.
    /// Returns 404 if the server belongs to another user.
    #[oai(path = "/mcp/servers/:id", method = "patch", tag = "ApiTags::Mcp")]
    async fn update_server(
        &self,
        id: Path<String>,
        body: Json<UpdateMcpServerRequest>,
        poem_req: &poem::Request,
    ) -> Result<Json<McpServer>> {
        let user_id = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;

        // Ownership gate before disconnect — otherwise a non-owner could
        // disrupt another user's running server even when the UPDATE
        // itself fails the WHERE clause.
        let owns: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM mcp_servers WHERE id = $1 AND user_id = $2")
                .bind(&id.0)
                .bind(&user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| {
                    poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
                })?;
        if owns.is_none() {
            return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
        }

        // First disconnect if connected
        {
            let manager = self.manager.write().await;
            manager.disconnect(&id.0).await.ok();
        }

        // Build dynamic update query with PostgreSQL $N placeholders
        let mut query = "UPDATE mcp_servers SET ".to_string();
        let mut updates = Vec::new();
        let mut param_idx = 1;

        if body.name.is_some() {
            updates.push(format!("name = ${}", param_idx));
            param_idx += 1;
        }
        if body.server_type.is_some() {
            updates.push(format!("server_type = ${}", param_idx));
            param_idx += 1;
        }
        if body.image_or_command.is_some() {
            updates.push(format!("image_or_command = ${}", param_idx));
            param_idx += 1;
        }
        if body.args.is_some() {
            updates.push(format!("args = ${}", param_idx));
            param_idx += 1;
        }
        if body.env_vars.is_some() {
            updates.push(format!("env_vars = ${}", param_idx));
            param_idx += 1;
        }

        // Always reset status to disconnected on update
        updates.push("status = 'disconnected'".to_string());
        updates.push("error_message = NULL".to_string());
        updates.push(format!("updated_at = ${}", param_idx));
        param_idx += 1;

        query.push_str(&updates.join(", "));
        // Scope the UPDATE to (id, user_id) so a TOCTOU between the
        // ownership check above and this query can't write to another
        // user's row.
        query.push_str(&format!(
            " WHERE id = ${} AND user_id = ${}",
            param_idx,
            param_idx + 1
        ));

        let mut q = sqlx::query(&query);

        if let Some(name) = &body.name {
            q = q.bind(name);
        }
        if let Some(server_type) = &body.server_type {
            q = q.bind(server_type);
        }
        if let Some(image) = &body.image_or_command {
            q = q.bind(image);
        }
        if let Some(args) = &body.args {
            q = q.bind(serde_json::to_string(args).unwrap_or_default());
        }
        if let Some(env_vars) = &body.env_vars {
            q = q.bind(serde_json::to_string(env_vars).unwrap_or_default());
        }

        let now = chrono::Utc::now().timestamp();
        q = q.bind(now);
        q = q.bind(&id.0);
        q = q.bind(&user_id);

        q.execute(&self.pool).await.map_err(|e| {
            eprintln!("Failed to update MCP server: {}", e);
            poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        // Fetch updated server to return — scoped to caller.
        let row = sqlx::query(
            r#"SELECT id, name, server_type, image_or_command, status, error_message,
                      created_at, args, env_vars
               FROM mcp_servers WHERE id = $1 AND user_id = $2"#,
        )
        .bind(&id.0)
        .bind(&user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| poem::Error::from_status(poem::http::StatusCode::NOT_FOUND))?;

        let args_str: Option<String> = row.get("args");
        let env_str: Option<String> = row.get("env_vars");

        Ok(Json(McpServer {
            id: row.get("id"),
            name: row.get("name"),
            server_type: row.get("server_type"),
            image_or_command: row.get("image_or_command"),
            status: row.get("status"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            args: args_str.and_then(|s| serde_json::from_str(&s).ok()),
            env_vars: env_str.and_then(|s| serde_json::from_str(&s).ok()),
        }))
    }

    /// Delete an MCP server. Multi-tenant: caller must own. Pre-fix this
    /// endpoint had NO user filter — any authenticated user could delete
    /// any other user's MCP server by id. Returns 404 (not 403) on
    /// non-ownership so existence isn't leaked.
    #[oai(path = "/mcp/servers/:id", method = "delete", tag = "ApiTags::Mcp")]
    async fn delete_server(
        &self,
        id: Path<String>,
        poem_req: &poem::Request,
    ) -> Result<Json<serde_json::Value>> {
        let user_id = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;

        // Ownership-guarded disconnect — don't disrupt another user's
        // running server just because we received a DELETE for that id.
        let owns: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM mcp_servers WHERE id = $1 AND user_id = $2")
                .bind(&id.0)
                .bind(&user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| {
                    poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
                })?;
        if owns.is_none() {
            return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
        }

        {
            let manager = self.manager.write().await;
            let _ = manager.disconnect(&id.0).await;
        }

        let result = sqlx::query("DELETE FROM mcp_servers WHERE id = $1 AND user_id = $2")
            .bind(&id.0)
            .bind(&user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Failed to delete MCP server: {}", e);
                poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        if result.rows_affected() == 0 {
            return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
        }

        eprintln!("Deleted MCP server: {}", id.0);

        Ok(Json(serde_json::json!({"deleted": true})))
    }

    /// Connect to an MCP server and discover tools. Multi-tenant: caller
    /// must own. The `env_vars` column can hold secrets (API tokens for
    /// the upstream MCP server), so the SELECT must be ownership-scoped.
    #[oai(
        path = "/mcp/servers/:id/connect",
        method = "post",
        tag = "ApiTags::Mcp"
    )]
    async fn connect_server(
        &self,
        id: Path<String>,
        poem_req: &poem::Request,
    ) -> Result<Json<ConnectResponse>> {
        let user_id = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;

        let row = sqlx::query(
            r#"SELECT id, name, server_type, image_or_command, args, env_vars
               FROM mcp_servers WHERE id = $1 AND user_id = $2"#,
        )
        .bind(&id.0)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Failed to load MCP server: {}", e);
            poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .ok_or_else(|| poem::Error::from_status(poem::http::StatusCode::NOT_FOUND))?;

        let server_id: String = row.get("id");
        let name: String = row.get("name");
        let server_type_str: String = row.get("server_type");
        let image_or_command: String = row.get("image_or_command");
        let args_str: Option<String> = row.get("args");
        let env_str: Option<String> = row.get("env_vars");

        let args: Vec<String> =
            serde_json::from_str(&args_str.unwrap_or_default()).unwrap_or_default();
        let env_vars: HashMap<String, String> =
            serde_json::from_str(&env_str.unwrap_or_default()).unwrap_or_default();

        let server_type = match server_type_str.as_str() {
            "docker" => ServerType::Docker,
            "stdio" => ServerType::Stdio,
            "sse" => ServerType::Sse,
            _ => {
                eprintln!("Unknown server type: {}", server_type_str);
                return Err(poem::Error::from_status(
                    poem::http::StatusCode::BAD_REQUEST,
                ));
            }
        };

        let config = McpServerConfig {
            id: server_id.clone(),
            name,
            server_type,
            image_or_command,
            args,
            env_vars,
        };

        sqlx::query("UPDATE mcp_servers SET status = 'connecting' WHERE id = $1")
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .ok();

        let manager = self.manager.write().await;
        match manager.connect(&config).await {
            Ok(tools) => {
                for tool in &tools {
                    let tool_id = Uuid::new_v4().to_string();
                    let schema_json = tool
                        .input_schema
                        .as_ref()
                        .map(|s| serde_json::to_string(s).unwrap_or_default());

                    sqlx::query(
                        r#"INSERT INTO mcp_tools (id, server_id, name, description, input_schema)
                           VALUES ($1, $2, $3, $4, $5)"#,
                    )
                    .bind(&tool_id)
                    .bind(&id.0)
                    .bind(&tool.name)
                    .bind(&tool.description)
                    .bind(&schema_json)
                    .execute(&self.pool)
                    .await
                    .ok();
                }

                sqlx::query("UPDATE mcp_servers SET status = 'connected', error_message = NULL WHERE id = $1")
                    .bind(&id.0)
                    .execute(&self.pool)
                    .await
                    .ok();

                eprintln!(
                    "Connected to MCP server {}, discovered {} tools",
                    id.0,
                    tools.len()
                );

                Ok(Json(ConnectResponse {
                    status: "connected".to_string(),
                    tools_discovered: tools.len(),
                }))
            }
            Err(e) => {
                let error_msg = e.to_string();
                sqlx::query(
                    "UPDATE mcp_servers SET status = 'error', error_message = $1 WHERE id = $2",
                )
                .bind(&error_msg)
                .bind(&id.0)
                .execute(&self.pool)
                .await
                .ok();

                eprintln!("Failed to connect to MCP server: {}", e);
                Err(poem::Error::from_string(
                    error_msg,
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }

    /// Disconnect from an MCP server. Multi-tenant: caller must own.
    #[oai(
        path = "/mcp/servers/:id/disconnect",
        method = "post",
        tag = "ApiTags::Mcp"
    )]
    async fn disconnect_server(
        &self,
        id: Path<String>,
        poem_req: &poem::Request,
    ) -> Result<Json<serde_json::Value>> {
        let user_id = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;

        let owns: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM mcp_servers WHERE id = $1 AND user_id = $2")
                .bind(&id.0)
                .bind(&user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| {
                    poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
                })?;
        if owns.is_none() {
            return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
        }

        let manager = self.manager.write().await;
        manager.disconnect(&id.0).await.ok();

        sqlx::query("DELETE FROM mcp_tools WHERE server_id = $1")
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query(
            "UPDATE mcp_servers SET status = 'disconnected' WHERE id = $1 AND user_id = $2",
        )
        .bind(&id.0)
        .bind(&user_id)
        .execute(&self.pool)
        .await
        .ok();

        eprintln!("Disconnected from MCP server: {}", id.0);

        Ok(Json(serde_json::json!({"status": "disconnected"})))
    }

    /// List tools from an MCP server. Multi-tenant: caller must own
    /// the server. Tools themselves are derived from the server, so
    /// returning them for someone else's server leaks the server's
    /// tool surface — gate at the server level.
    #[oai(path = "/mcp/servers/:id/tools", method = "get", tag = "ApiTags::Mcp")]
    async fn list_tools(
        &self,
        id: Path<String>,
        poem_req: &poem::Request,
    ) -> Result<Json<Vec<McpToolResponse>>> {
        let user_id = poem_req
            .extensions()
            .get::<mawi_core::auth::User>()
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                poem::Error::from_string(
                    "Authentication required",
                    poem::http::StatusCode::UNAUTHORIZED,
                )
            })?;

        let owns: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM mcp_servers WHERE id = $1 AND user_id = $2")
                .bind(&id.0)
                .bind(&user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| {
                    poem::Error::from_status(poem::http::StatusCode::INTERNAL_SERVER_ERROR)
                })?;
        if owns.is_none() {
            return Err(poem::Error::from_status(poem::http::StatusCode::NOT_FOUND));
        }

        let rows = sqlx::query(
            r#"SELECT id, server_id, name, description, input_schema
               FROM mcp_tools WHERE server_id = $1"#,
        )
        .bind(&id.0)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let response: Vec<McpToolResponse> = rows
            .into_iter()
            .map(|row| {
                let schema_str: Option<String> = row.get("input_schema");
                McpToolResponse {
                    id: row.get("id"),
                    server_id: row.get("server_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    input_schema: schema_str.and_then(|s| serde_json::from_str(&s).ok()),
                }
            })
            .collect();

        Ok(Json(response))
    }
}
