-- Link services to allowed MCP servers (MERGED FROM OLD 023/019)

-- MCP Server Configurations
CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    server_type TEXT NOT NULL CHECK (server_type IN ('docker', 'stdio', 'sse')),
    image_or_command TEXT NOT NULL,
    args TEXT DEFAULT '[]',
    env_vars TEXT DEFAULT '{}',
    status TEXT DEFAULT 'disconnected' CHECK (status IN ('connected', 'disconnected', 'error', 'connecting')),
    error_message TEXT,
    last_heartbeat TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Discovered tools from MCP servers
CREATE TABLE IF NOT EXISTS mcp_tools (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    input_schema TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
);

-- Link MCP tools to services (many-to-many)
CREATE TABLE IF NOT EXISTS service_mcp_tools (
    service_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (service_id, tool_id),
    FOREIGN KEY (service_id) REFERENCES services(name) ON DELETE CASCADE,
    FOREIGN KEY (tool_id) REFERENCES mcp_tools(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_mcp_servers_user ON mcp_servers(user_id);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_status ON mcp_servers(status);
CREATE INDEX IF NOT EXISTS idx_mcp_tools_server ON mcp_tools(server_id);
CREATE INDEX IF NOT EXISTS idx_mcp_tools_name ON mcp_tools(name);
CREATE INDEX IF NOT EXISTS idx_service_mcp_tools_service ON service_mcp_tools(service_id);

-- Link services to allowed MCP servers
CREATE TABLE IF NOT EXISTS service_mcp_servers (
    service_name TEXT NOT NULL,
    mcp_server_id TEXT NOT NULL,
    created_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    PRIMARY KEY (service_name, mcp_server_id),
    FOREIGN KEY (service_name) REFERENCES services(name) ON DELETE CASCADE,
    FOREIGN KEY (mcp_server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
);

-- Index for efficient lookup during tool execution
CREATE INDEX idx_service_mcp_servers_lookup ON service_mcp_servers(service_name, mcp_server_id);
