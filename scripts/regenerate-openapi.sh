#!/usr/bin/env bash
#
# regenerate-openapi.sh — refresh the static OpenAPI spec.
#
# The gateway serves its OpenAPI spec at runtime via /spec, but having a
# committed openapi.json in the repo means:
#   - downstream SDK generators (sdk/node, sdk/python) can run offline
#   - PRs that change the API surface produce a visible diff in review
#   - third parties (Postman, Insomnia) can import the spec without a
#     running gateway
#
# Workflow: run this script after any change to backend/gateway/src/api.rs
# (or any *_api.rs that exposes new routes). Commit the resulting
# openapi.json alongside your API change. The mawi-client / mawi-cli /
# mawi-mcp typed wrappers in `backend/client/src/lib.rs` should be
# updated in the same PR so all four (server, client, CLI, MCP) advance
# in lockstep.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-openapi.json}"

echo "→ building gateway crate..."
cargo build --quiet --manifest-path backend/Cargo.toml --bin print_openapi

echo "→ generating spec..."
cargo run --quiet --manifest-path backend/Cargo.toml --bin print_openapi > "$OUT"

# Pretty-print so diffs are readable. jq is widely available; fall back
# to leaving the raw output if it isn't installed.
if command -v jq >/dev/null 2>&1; then
    jq '.' "$OUT" > "$OUT.pretty" && mv "$OUT.pretty" "$OUT"
fi

LINES=$(wc -l < "$OUT" | tr -d ' ')
PATHS=$(grep -c '"/' "$OUT" || true)
echo "→ wrote $OUT ($LINES lines, ~$PATHS paths)"

echo
echo "Next steps:"
echo "  - Commit $OUT alongside your API change."
echo "  - If the API gained new fields or routes, update typed wrappers"
echo "    in backend/client/src/lib.rs so mawi CLI + mawi-mcp stay aligned."
