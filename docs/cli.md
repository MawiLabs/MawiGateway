# `mg` CLI

A command-line interface for MawiGateway. Manage providers, models,
services, MCP servers, and API keys from a terminal or CI script.

The CLI talks to the gateway's REST API via the shared `mawi-client`
crate. When the API surface changes, you only update one file
(`backend/client/src/lib.rs`) and the CLI rebuilds against the new
types — so the CLI and the gateway never drift.

## Install

```bash
# from a clone of the repo
cd backend
cargo install --path cli

# or build a release binary in place
cargo build --release --bin mg
ls target/release/mg
```

The binary is named `mg` and lands in `~/.cargo/bin` if you used
`cargo install`.

## Authenticate

Pick one (in order of precedence):

```bash
# 1. flag (best for one-off commands and CI)
mg --api-key sk_live_... whoami

# 2. environment variable
export MG_API_KEY=sk_live_...
mg whoami

# 3. saved config file (~/.mawigateway/config.yaml, mode 0600)
mg auth login --gateway-url http://localhost:8030
# prompts for the API key
```

Generate a key from the UI (`/governance/access-control` →
"Generate API Key") or with the CLI itself once you've authenticated
once:

```bash
mg keys create --name "my-laptop"
```

## Commands

### Providers

```bash
mg providers list
mg providers add --name "OpenAI Prod" --type openai --api-key sk-...
mg providers add --name "Self-hosted Ollama" --type selfhosted \
    --endpoint http://localhost:11434
mg providers remove <provider-uuid>
```

### Models

```bash
mg models list
mg models add --name gpt-4o-mini --provider <provider-uuid> --modality text
mg models add --name dall-e-3 --provider <provider-uuid> --modality image
mg models remove <model-uuid>
```

### Services

```bash
mg services list
mg services create --name text-default --type POOL \
    --strategy least_cost --modality text \
    --models <id1>,<id2>,<id3>
mg services delete text-default
```

Strategies: `weighted_random`, `least_cost`, `least_latency`, `health`,
`none`, `planner` (agentic services use `planner`).

### API keys

```bash
mg keys list
mg keys create --name "ci-pipeline"   # raw key shown ONCE
mg keys revoke <key-id>
```

### MCP servers (consumed by the gateway)

```bash
mg mcp list
mg mcp add --name "github" --type docker \
    --image-or-command ghcr.io/github/github-mcp-server
mg mcp connect <server-uuid>
mg mcp remove <server-uuid>
```

### Logs and analytics

```bash
mg logs --limit 100
mg analytics --range 24h     # 24h | 7d | 30d
```

### One-shot chat (smoke test)

```bash
mg chat --service text-default "what's 2+2?"
mg chat --service text-default "summarize this" --max-tokens 200
```

### YAML config

The same schema as `mawigateway.yaml` — apply it with:

```bash
mg config validate ./my-stack.yaml   # local lint, no network
mg config apply ./my-stack.yaml      # POST to /v1/config/apply
```

### Diagnostics

```bash
mg whoami           # who is this CLI authenticated as?
mg doctor           # gateway URL + auth source + connection probe
mg --json <cmd>     # any command emits JSON for piping into jq
```

## Using the CLI in CI

```yaml
# .github/workflows/deploy.yml
- name: Configure gateway
  env:
    MG_API_KEY: ${{ secrets.MG_API_KEY }}
    MG_GATEWAY_URL: https://gw.your-prod.example.com
  run: |
    mg config apply ./infra/mawigateway.yaml
    mg services list --json > services.json
```

`--json` output on every command means you can pipe through `jq` or
parse with any language.

## When the API changes

1. Update the gateway code in `backend/gateway/src/*.rs`.
2. Run `./scripts/regenerate-openapi.sh` — refreshes `openapi.json`.
3. Update typed wrappers in `backend/client/src/lib.rs` to match.
4. The CLI rebuild (`cargo build`) picks up the new types
   automatically; failing builds tell you what's stale.
5. Add or rename CLI subcommands in `backend/cli/src/main.rs` if a new
   capability needs surface area.

The whole point of this layout is that step 3 is the only manual sync
between the server and its CLI — and it's a single file.
