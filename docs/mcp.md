# `mawi-mcp` — MawiGateway as an MCP server

`mawi-mcp` exposes MawiGateway to AI agents over the **Model Context
Protocol** (MCP). Drop it in your agent's MCP config and the agent can
list services, route chat completions, inspect logs, and (optionally)
provision providers/models — all through typed tool calls.

This is the strategic move that turns MawiGateway from "a thing
developers configure" into "a thing every AI agent can use." Claude
Code, Cursor, Cline and any other MCP host can now route requests
through your gateway without ever touching its REST API directly.

## Install

```bash
cd backend
cargo install --path mcp
```

The binary lands at `~/.cargo/bin/mawi-mcp` (add to PATH if needed).

## Wire it into your agent

### Claude Code

Edit `~/.config/claude/mcp.json` (or `~/.claude/mcp.json`,
platform-dependent):

```jsonc
{
  "mcpServers": {
    "mawigateway": {
      "command": "mawi-mcp",
      "env": {
        "MG_API_KEY": "sk_live_...",
        "MG_GATEWAY_URL": "http://localhost:8030"
      }
    }
  }
}
```

### Cursor

Add to `~/.cursor/mcp.json`:

```jsonc
{
  "mcpServers": {
    "mawigateway": {
      "command": "mawi-mcp",
      "env": { "MG_API_KEY": "sk_live_..." }
    }
  }
}
```

### Cline / other hosts

Same shape. Any host that supports stdio transport works.

After editing, restart your agent. It should advertise the new tools
under a "mawigateway" namespace.

## Tools the agent can call

### Read tools (always available)

| Tool | What it does |
|------|--------------|
| `mawi_whoami` | Confirm credentials. Returns user id, email, tier. |
| `mawi_list_providers` | Configured AI providers. |
| `mawi_list_models` | Models with modality and health. |
| `mawi_list_services` | Routing services — names you can pass to `mawi_chat`. |
| `mawi_list_mcp_servers` | MCP servers the gateway is consuming. |
| `mawi_get_logs` | Recent request logs (debug failures). |
| `mawi_get_analytics` | Usage stats over `24h` / `7d` / `30d`. |

### Inference

| Tool | What it does |
|------|--------------|
| `mawi_chat` | Send messages to a service. Gateway picks the underlying model based on the service's strategy (least_cost, least_latency, planner, etc.). |

### Write tools (gated, off by default)

These require `--allow-writes` (or `MG_MCP_ALLOW_WRITES=1`) so a
misbehaving agent can't reconfigure your gateway:

| Tool | What it does |
|------|--------------|
| `mawi_register_provider` | Add a provider (OpenAI, Anthropic, …). |
| `mawi_create_model` | Register a model under a provider. |
| `mawi_create_service` | Create a routing pool or agentic service. |
| `mawi_create_api_key` | Generate a new API key. |

To enable:

```jsonc
{
  "mcpServers": {
    "mawigateway": {
      "command": "mawi-mcp",
      "args": ["--allow-writes"],
      "env": { "MG_API_KEY": "sk_live_..." }
    }
  }
}
```

## Try it without an agent

Use [`mcp-inspector`](https://github.com/modelcontextprotocol/inspector)
or send raw JSON-RPC over stdio:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  MG_API_KEY=sk_live_... mawi-mcp

echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | \
  MG_API_KEY=sk_live_... mawi-mcp
```

You should see the tool catalog in the response.

## Architecture

```
┌─────────────────┐    JSON-RPC over    ┌──────────────┐
│  AI agent       │     stdio           │  mawi-mcp    │
│  (Claude Code,  │  ←───────────────→  │  (this bin)  │
│   Cursor, etc.) │                     └──────┬───────┘
└─────────────────┘                            │
                                               │ HTTP
                                               ▼
                                        ┌──────────────┐
                                        │ MawiGateway  │
                                        │   (REST)     │
                                        └──────────────┘
```

`mawi-mcp` is a thin translator. It does NOT cache, queue, or buffer.
Every tool call is a pass-through to the gateway's REST API via the
shared `mawi-client` crate.

That's deliberate: when the API surface changes, only `mawi-client`
needs to update; `mawi-mcp` and `mawi` (the CLI) rebuild against the
new types automatically. There's exactly one place in the codebase
where the API contract lives, and it's not duplicated three times.

## Authentication

`mawi-mcp` reads `MG_API_KEY` at startup. The key flows through every
tool call as a `Bearer` token. No per-tool key prompts — agents never
see the raw key.

If `MG_API_KEY` is unset, the server starts anyway (so MCP hosts can
list tools) but every tool call returns an error pointing to the
missing key.

## Why JSON-RPC over stdio (not SSE/HTTP)?

Every major MCP host today (Claude Code, Cursor, Cline,
mcp-inspector) supports stdio. SSE/HTTP transports are in the spec but
have fragmented adoption. Starting with stdio means we work everywhere
day-one. SSE can be added later — the tool definitions stay
identical, only the framing changes.

## When the API changes

1. Update the gateway code (e.g. add a new endpoint).
2. Run `./scripts/regenerate-openapi.sh`.
3. Add a typed method in `backend/client/src/lib.rs`.
4. Add a tool definition + dispatch arm in `backend/mcp/src/main.rs`.
5. Restart your agent — `tools/list` will show the new tool.

Steps 3 and 4 are the only manual sync points. Steps 1, 2, and 5 are
mechanical. We chose this layout precisely so an AI agent (the one
consuming this MCP server) can read the diff of `mawi-client` and
write the matching MCP tool dispatch arm itself.
