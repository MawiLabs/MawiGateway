# Calling MawiGateway from OpenAI / Anthropic SDKs

> **Design point:** MawiGateway routes through **services**, not
> models directly. A service is a named pool with a routing strategy
> (least_cost, least_latency, weighted_random, planner). Services are
> the surface where you control cost, fallback, budgets, caching, and
> guardrails. Letting clients address models directly would bypass
> all of that — so the gateway requires a `service` field.

This page explains how to call MawiGateway from any OpenAI-compatible
SDK (the Python `openai` package, the Node `openai` package, LangChain,
LlamaIndex, etc.) without losing the service abstraction.

## What's already compatible

These match OpenAI's API exactly — no adapter needed:

| Surface | Compat |
|--------|-------|
| Endpoint | `POST /v1/chat/completions` ✅ |
| Auth | `Authorization: Bearer <key>` ✅ |
| Message shape | `{role, content}` ✅ |
| Response shape | `{id, object, created, model, choices, usage}` ✅ |
| Streaming | SSE `data: {...}\n\n` ✅ |
| OpenAPI spec | Published at `/swagger-ui` and `/spec` ✅ |

What's **different** by design:

| Surface | OpenAI | MawiGateway |
|--------|--------|-------------|
| Routing field | `model` (e.g. `"gpt-4o"`) | `service` (e.g. `"text-default"`) |
| Error body | `{"error":{"message":"...","type":"...","code":"..."}}` | `"plain string"` (will move to OpenAI shape — tracked) |

## Calling from the OpenAI Python SDK

The OpenAI client doesn't know about a `service` field. Two clean
approaches:

### Approach 1 — direct HTTP (simplest)

The OpenAI client's underlying `httpx` is exposed; pass `extra_body`:

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://gw.your-domain.com/v1",
    api_key="sk_live_...",   # MawiGateway API key, not an OpenAI key
)

resp = client.chat.completions.create(
    model="ignored-but-required-by-sdk",   # SDK insists on this field
    messages=[{"role": "user", "content": "hello"}],
    extra_body={"service": "text-default"},  # ← the actual routing key
)
```

Same pattern works for the Node SDK with the `query` / `body` overrides
and any SDK that lets you splice in extra body keys.

### Approach 2 — `mawi-client` (Python or Node)

Auto-generated SDK at `sdk/python/` and `sdk/node/`. These accept
`service` as a first-class field:

```python
from ma_wi_api_client import Client
from ma_wi_api_client.api.chat import post_chat_completions

client = Client(base_url="https://gw.your-domain.com/v1") \
    .with_headers({"Authorization": "Bearer sk_live_..."})

resp = post_chat_completions.sync_detailed(
    client=client,
    body={
        "service": "text-default",
        "messages": [{"role": "user", "content": "hello"}],
    },
)
```

### Approach 3 — `mawi` CLI (one-shot or scripts)

```bash
mawi chat "hello world" --service text-default
```

For shell scripts and CI. See `docs/cli.md`.

## What about LangChain / LlamaIndex?

Both have `ChatOpenAI` constructors that take a `model_kwargs` dict.
Pass `service` there:

```python
# LangChain
from langchain_openai import ChatOpenAI
llm = ChatOpenAI(
    base_url="https://gw.your-domain.com/v1",
    api_key="sk_live_...",
    model="ignored",
    model_kwargs={"service": "text-default"},
)
```

## Why not silently accept `model` as a service name?

We considered it (and shipped it briefly during a development sprint).
The reason we backed out:

If `model: "gpt-4o"` silently routed to a service literally named
`"gpt-4o"`, you'd get a fast win for adoption — but every benefit of
the gateway disappears. Pools wouldn't pool, strategies wouldn't
strategize, budgets wouldn't enforce, caching wouldn't apply, planners
wouldn't plan. Users would rename their services to OpenAI model names
to make the SDKs work and lose the routing layer they pay you for.

Services are the primitive on purpose. The product is the routing,
not "another OpenAI key with extra latency."

## Service aliases — the right OpenAI compat answer (shipped)

Operators who *want* OpenAI client compat declare aliases on a service:

```bash
mawi services create \
    --name text-default \
    --type POOL \
    --strategy least_cost \
    --aliases gpt-4o,gpt-4-turbo,claude-3-5-sonnet \
    --models <id1>,<id2>,<id3>
```

OpenAI clients sending `service: "gpt-4o"` (via `extra_body`) now route
to `text-default` — picking up the pool, the strategy, the cache, the
budget, all of it. The operator explicitly listed `gpt-4o` as an alias
on this service, so this is consent, not a silent fallback.

**How it works:**
- `services.aliases` is a `TEXT[]` column with a GIN index for fast
  reverse lookup.
- The chat handler resolves `request.service` against
  `WHERE name = $1 OR $1 = ANY(aliases) LIMIT 1`.
- A trigger enforces uniqueness across the namespace: an alias can't
  collide with another service's name *or* with another service's
  alias. The gateway returns `409 Conflict` with the colliding name.
- The cache is alias-aware: a hit by alias is cached under both the
  alias and the canonical name so subsequent lookups via either are
  fast.

**Putting it together — full OpenAI drop-in:**

```python
from openai import OpenAI

# 1. Operator created service "text-default" with aliases ["gpt-4o", ...]
# 2. Client just specifies the alias as the service:
client = OpenAI(
    base_url="https://gw.your-domain.com/v1",
    api_key="sk_live_...",
)
resp = client.chat.completions.create(
    model="ignored-by-mawi",
    messages=[{"role": "user", "content": "hello"}],
    extra_body={"service": "gpt-4o"},   # alias of text-default
)
```

Routing strategy, caching, planners, budgets — all run as configured
on `text-default`, transparently to the client.

**Multiple aliases per service** is fine. Re-using the same alias
across two services is rejected at write time:

```
$ mawi services create --name b --aliases gpt-4o
✗ alias gpt-4o already used by service a
```

## Error shape (in flight)

Today errors return `200 OK` with a JSON-string body for known errors
(`"missing field: service"`) and a 5xx for unexpected ones. OpenAI
returns a structured object:

```json
{ "error": { "message": "...", "type": "invalid_request_error", "code": null } }
```

Bringing MawiGateway error responses to that shape is a tracked
follow-up. SDK error handling will improve when this lands.

## Authentication notes

- **Generate a MawiGateway key**, don't use an OpenAI key:
  - UI: `/governance/access-control` → Generate API Key
  - CLI: `mawi keys create --name laptop`
  - The raw key is shown once; store it in your secret manager.
- **Provider keys** (your OpenAI / Anthropic / Azure keys) live inside
  MawiGateway, not in your client. The gateway uses them when routing
  through services.

## Quick smoke test

```bash
curl https://gw.your-domain.com/v1/chat/completions \
  -H "Authorization: Bearer sk_live_..." \
  -H "Content-Type: application/json" \
  -d '{
    "service": "text-default",
    "messages": [{"role": "user", "content": "hello"}]
  }'
```

If this returns an OpenAI-shaped completion, your gateway is wired up.
The `service` field is required — no `service` value in your services
table will return a 400 with the friendly error you saw above.
