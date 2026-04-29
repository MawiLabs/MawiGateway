# `mawi` CLI

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
cargo build --release --bin mawi
ls target/release/mawi
```

The binary is named `mawi` and lands in `~/.cargo/bin` if you used
`cargo install`.

## Authenticate

Pick one (in order of precedence):

```bash
# 1. flag (best for one-off commands and CI)
mawi --api-key sk_live_... whoami

# 2. environment variable
export MG_API_KEY=sk_live_...
mawi whoami

# 3. saved config file (~/.mawi/config.yaml, mode 0600)
mawi auth login --gateway-url http://localhost:8030
# prompts for the API key
```

Generate a key from the UI (`/governance/access-control` →
"Generate API Key") or with the CLI itself once you've authenticated
once:

```bash
mawi keys create --name "my-laptop"
```

## Commands

### Providers

```bash
mawi providers list
mawi providers add --name "OpenAI Prod" --type openai --api-key sk-...
mawi providers add --name "Self-hosted Ollama" --type selfhosted \
    --endpoint http://localhost:11434
mawi providers remove <provider-uuid>
```

### Models

```bash
mawi models list
mawi models add --name gpt-4o-mini --provider <provider-uuid> --modality text
mawi models add --name dall-e-3 --provider <provider-uuid> --modality image
mawi models remove <model-uuid>
```

### Services

```bash
mawi services list
mawi services create --name text-default --type POOL \
    --strategy least_cost --modality text \
    --models <id1>,<id2>,<id3>
mawi services delete text-default
```

Strategies: `weighted_random`, `least_cost`, `least_latency`, `health`,
`none`, `planner` (agentic services use `planner`).

### API keys

```bash
mawi keys list
mawi keys create --name "ci-pipeline"   # raw key shown ONCE
mawi keys revoke <key-id>
```

### MCP servers (consumed by the gateway)

```bash
mawi mcp list
mawi mcp add --name "github" --type docker \
    --image-or-command ghcr.io/github/github-mcp-server
mawi mcp connect <server-uuid>
mawi mcp remove <server-uuid>
```

### Logs and analytics

```bash
mawi logs --limit 100
mawi analytics --range 24h     # 24h | 7d | 30d
```

### One-shot chat (smoke test)

```bash
mawi chat --service text-default "what's 2+2?"
mawi chat --service text-default "summarize this" --max-tokens 200
```

### YAML config

The same schema as `mawigateway.yaml` — apply it with:

```bash
mawi config validate ./my-stack.yaml   # local lint, no network
mawi config apply ./my-stack.yaml      # POST to /v1/config/apply
```

### Diagnostics

```bash
mawi whoami           # who is this CLI authenticated as?
mawi doctor           # gateway URL + auth source + connection probe
mawi --json <cmd>     # any command emits JSON for piping into jq
```

## Using the CLI in CI

```yaml
# .github/workflows/deploy.yml
- name: Configure gateway
  env:
    MG_API_KEY: ${{ secrets.MG_API_KEY }}
    MG_GATEWAY_URL: https://gw.your-prod.example.com
  run: |
    mawi config apply ./infra/mawigateway.yaml
    mawi services list --json > services.json
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
