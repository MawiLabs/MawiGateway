# MaWi Gateway

**Open source AI Agentic Gateway** - Unified API interface that provides access to multiple AI Models and MCPs (OpenAI, Azure, Gemini, Anthropic, etc.). It intelligently routes requests, manages quota, handles authentication, and provides a consistent interface for multimodal AI capabilities including chat, image generation, video generation, and audio processing.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](docker-compose.yml)

<img src="docs/images/system-overview.png" alt="System Overview" width="800">

## 🌟 Always Open Source

MaWi Gateway core will **always be free and open source**. Team and Enterprise editions will add exclusive features like multi-user collaboration and SSO, but the core functionality remains free forever.

## ✨ Features (Community Edition)

- 🔌 **Multi-Provider Support** — across every modality (see matrix below)
- 🎮 **Interactive Playground** — test + compare models in real-time
  
<img src="docs/images/playground.png" alt="Playground" width="800">

- 📊 **Analytics & Logging** — track usage, costs, and performance
- 🌐 **Multimodal Support** — chat, image, audio, video — one gateway
- 🔗 **MCP Server Integration** — Model Context Protocol support
- ⚙️ **Service Management** — pools, failover, weighted routing, circuit breakers
- 🛡️ **Governance** — access control + guardrails (view-only in Community)

### Provider matrix

| Modality | Providers |
|---------|-----------|
| **Chat / text** | OpenAI · Anthropic · Google Gemini · Azure OpenAI · xAI · Mistral · Perplexity · DeepSeek · OpenRouter · self-hosted (Ollama / vLLM / llama.cpp) |
| **Image** | OpenAI (DALL·E, gpt-image) · xAI (Grok Imagine) · Google (Imagen via Gemini) · Azure |
| **Video** | OpenAI (Sora 2) · Google (Veo 3) · Runway (Gen-3 Alpha) · Kling · Luma (Dream Machine) · Pika · MiniMax (Hailuo) · ByteDance (Seedance) |
| **Voice (TTS)** | OpenAI · ElevenLabs · Hume (Octave) |
| **Speech-to-text** | OpenAI (Whisper) · ElevenLabs |
| **Music** | ElevenLabs Music _(more coming)_ |

API keys live as env vars (e.g. `MG_OPENAI_API_KEY`, `MG_RUNWAY_API_KEY`) or encrypted in the providers table — see [`.env.example`](backend/.env.example).

## 🚀 Quick Start

### Using Docker Compose (Recommended)

```bash
# Clone the repository
git clone https://github.com/mawi-ai/mawi.git
cd mawi

# Start all services
docker compose up -d

# Access the UI
open http://localhost:3001
```

### Pre-flight checklist (catches the top 5 setup mistakes — #65)

Before `docker compose up -d`, verify each:

- [ ] **Master key generated.** `MG_MASTER_KEY` set in `.env` (32 hex chars):
      `openssl rand -hex 32 >> .env` then prefix `MG_MASTER_KEY=`. Without
      it the gateway can't decrypt provider API keys at boot and refuses
      to start.
- [ ] **`MG_DATABASE_URL` points at a fresh DB** — defaults are
      `postgres://mawi:password@mawi-postgres:5432/mawi`. If you're
      pointing at an existing DB, ensure migrations 001–037 have been
      applied (run `cargo run -p mawi-cli -- migrate` from `backend/`).
- [ ] **CORS origins listed.** `MG_CORS_ALLOWED_ORIGINS=http://localhost:3001`
      (or your frontend's hostname). Without this the admin UI's fetch
      calls 401 with no error in the browser console.
- [ ] **Ports free.** `8030` (API), `3001` (admin UI), `5432` (Postgres)
      must be available. `lsof -i :8030 -i :3001 -i :5432` to check.
- [ ] **At least one provider key set.** Even `MG_OPENAI_API_KEY` alone is
      enough to boot — `text-default` and `image-default` services will
      route there. Without any key, services are healthy at boot but every
      call 500s with "no API key configured" (#109's pre-flight).

After boot, hit `GET http://localhost:8030/v1/version` to confirm the
build SHA + version, then `GET /health` for liveness.

### Manual Setup

```bash
# Backend
cd backend
cargo build --release
./target/release/gateway

# Frontend
cd frontend
npm install
npm run dev
```

## 💾 Backup & restore (#64)

The gateway's stateful surface is **the Postgres DB only** — providers,
models, services, request_logs, model_health, encrypted credentials.
Everything else is reproducible from container images + `.env`.

### Daily snapshot (Docker Compose)

```bash
docker exec mawi-postgres pg_dump -U mawi -Fc mawi \
  > backups/mawi-$(date +%Y-%m-%d).dump
```

`-Fc` is the custom-format dump — smaller, supports parallel restore,
and works with `pg_restore --jobs=4`.

### Restore on a fresh DB

```bash
# 1. Bring up an empty DB
docker compose up -d mawi-postgres
# 2. Wait for it to be healthy
docker exec mawi-postgres pg_isready -U mawi
# 3. Drop + recreate to ensure clean state (DESTRUCTIVE — only on a
#    confirmed-fresh target)
docker exec mawi-postgres psql -U mawi -c 'DROP DATABASE IF EXISTS mawi; CREATE DATABASE mawi;'
# 4. Restore from the snapshot
docker exec -i mawi-postgres pg_restore -U mawi -d mawi --jobs=4 \
  < backups/mawi-2026-05-06.dump
# 5. Boot the gateway — it'll re-apply any newer migrations on top
docker compose up -d mawi-api
```

### Critical: keep `MG_MASTER_KEY` with the backup

Provider API keys in the dump are AES-GCM encrypted under
`MG_MASTER_KEY`. Restoring to a host with a *different* master key
makes every provider unusable until you re-paste each key in the
admin UI. **Store the master key in your password manager next to
the dump.**

### Production: managed Postgres

For prod, point `MG_DATABASE_URL` at a managed instance (RDS / Cloud
SQL / Supabase / Neon) and use the provider's PITR / scheduled
snapshots. The gateway has no special backup needs beyond standard
Postgres.

## 📚 Documentation

- [Self-Hosting Guide](docs/self-hosting.md)
- [API Reference](docs/api-reference.md)
- [Contributing](CONTRIBUTING.md)

## 🔜 Coming Soon

- 👥 **Team Edition** - Multi-user collaboration, shared workspaces
- 🏢 **Enterprise Edition** - SSO, audit logs, SLAs, dedicated support

## 🤝 Contributing <3

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📝 License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## ®️ Trademarks

The name "MaWi" and the MaWi logo are trademarks of the MaWi project. You may not use the name or logo in a way that suggests endorsement or affiliation without prior written consent.

## 🙏 Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Backend
- [Next.js](https://nextjs.org/) - Frontend
- [PostgreSQL](https://www.postgresql.org/) - Database
