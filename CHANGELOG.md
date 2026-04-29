# Changelog

All notable changes to MaWi Gateway will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-04-30

Major release: image / video / audio providers, retry-aware failover,
service aliases, semantic cache, audit log, API key scopes, full UI
overhaul, and a workspace-wide rename to the `MG_` namespace.

### Added

#### New providers
- **xAI Grok Imagine** — image + video generation (extends existing xAI adapter)
- **Runway Gen-4 / Gen-4.5** — text-to-video, async job model
- **Kuaishou Kling** — video, JWT bearer auth
- **Luma AI Dream Machine** — Ray-2 / Ray-Flash-2 video
- **Pika Labs** — Pika 2.2 video
- **MiniMax** — Hailuo video + Abab chat
- **ByteDance Seedance** — video via Volcano Ark (CN region)
- **Hume AI** — Octave TTS, fully aligned with the official `POST /tts` contract

#### Routing, reliability, observability
- Retry-aware failover gate — `RateLimit / Timeout / Unavailable / Internal`
  fail over to the next model; `Unauthorized / BadRequest / Misconfigured`
  fail fast with the upstream status preserved
- Typed `ProviderError` taxonomy on every adapter — `Retry-After` propagates
  through 429 and 503
- Distributed rate limiting via Redis (#79, opt-in via `MG_RATE_LIMIT_REDIS_URL`)
- Append-only audit log with read API (#80)
- OpenAI-shape error envelopes for SDK compatibility (#84)
- Semantic cache (exact-hash + pgvector tier, opt-in per service, #6)
- Service aliases — route OpenAI/Anthropic SDK requests through familiar
  model names like `gpt-4o` / `claude-sonnet-4-5` (#90)
- API key scopes — `admin / read / chat / chat:<service> / config:read /
  config:write` (#78)
- `Idempotency-Key` support extended to image / TTS / STT / S2S endpoints

#### Provider env-var fallback
- Any provider whose `api_key` column is empty falls back to `MG_<PROVIDER>_API_KEY`
  in the process environment — convenient for self-hosters who skip the admin UI

#### CLI / MCP / Client
- New `mawi-client` Rust crate — shared types for CLI, MCP, future SDKs
- New `mg` CLI binary (renamed from `mawi`)
- New `mg-mcp` MCP server (renamed from `mawi-mcp`) exposing the gateway
  to Claude Code, Cursor, Cline as `mg_chat`, `mg_list_*`, etc.

#### UI
- Premium pages across `/providers`, `/services`, `/providers/mcp`,
  `/analytics`, `/logs`, `/governance/*`, `/profile`, `/docs` —
  consistent breadcrumb + 4xl gradient H1 + tabular subtitle pattern,
  Skeleton loading, gradient cards with corner radial-glow on hover
- New provider catalog includes all 17 providers across foundation /
  hosted / audio / video / self-hosted categories
- Capability-locked model picker — provider determines the modality
  options (Kling can only be set to `video`, ElevenLabs to audio paths)
- Native datalist autocomplete on model names with inline catalog hint
- New `ChipInput` and `Select` UI primitives

### Changed (BREAKING)
- All gateway-owned env vars renamed to the `MG_` prefix —
  `DATABASE_URL` → `MG_DATABASE_URL`, `MAWI_MASTER_KEY` → `MG_MASTER_KEY`,
  `RUST_LOG` → `MG_RUST_LOG`, plus every provider key
  (`OPENAI_API_KEY` → `MG_OPENAI_API_KEY`, etc.). No backward-compat
  fallback — `.env.example` is the authoritative reference.
- CLI binary renamed `mawi` → `mg`
- MCP binary renamed `mawi-mcp` → `mg-mcp`
- MCP tool names renamed `mawi_*` → `mg_*`
- CLI config dir renamed `~/.mawi/` → `~/.mawigateway/`
- Postgres image switched to `pgvector/pgvector:pg15` so migrations apply
  cleanly on first boot (semantic cache requires the `vector` extension)
- Migration 036 is a no-op — providers are 100% user-managed (UI / yaml / API)

### Fixed
- OpenAI `stream_chat` now classifies non-2xx into typed errors instead of
  returning an empty stream — rate limits on streaming chat were silently
  masked before
- Hume adapter aligned with the official contract: `voice` payload sends
  `{name, provider}` (HUME_AI / CUSTOM_VOICE), output format selectable via
  `model="octave?format=wav"`, response content-type derived from the
  actual `encoding.format` Hume returned
- `scopes::validate_scope` now rejects uppercase service names per the
  doc-comment intent (was case-insensitive)
- Develop-broken state where `executor.rs` referenced `services.aliases`
  + `cache_*` columns from migrations that were sitting untracked

### Removed
- The non-functional Hume `/evi/chat` REST shim. Real EVI is bidirectional
  WebSocket — tracked in #101

### Tests
- 33 new tests; **93 total passing** across `mawi-core` lib + integration +
  `gateway` lib (0 failed)
- Per-adapter classification coverage (400/401/403/408/422/429/500/502/503,
  Retry-After parsing)
- Failover gate decision matrix locked down per `ProviderError` variant
- `env_api_key_for` alias resolution covered
- Hume request-body shape pinned against the published API contract

## [0.1.0] - 2026-01-21

### Added
- Initial release
- Community Edition (free and open source)
- Open source Community Edition
- Organization setup during registration
- Multi-provider support (OpenAI, Anthropic, Google, Azure, Ollama)
- Interactive Playground
- Analytics and logging
- Multimodal support (GPT-5, images, audio, video)
- MCP Server integration
- Service management (pools, failover, load balancing)
- Governance features (Access Control, Guardrails)
- Single Community tier (free)
- Professional open source documentation
