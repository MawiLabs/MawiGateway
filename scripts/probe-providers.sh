#!/usr/bin/env sh
# Provider key smoke tester.
#
# Usage: docker run --rm --env-file .env curlimages/curl:latest sh /probe.sh
#
# For each MG_*_API_KEY that's set in the environment, hits the cheapest
# auth-verification endpoint the provider exposes — usually a list-models
# call that doesn't bill against the account. Prints provider · status ·
# verdict with no key material echoed anywhere.
#
# Verdicts:
#   200/2xx          → key works
#   401/403          → key is invalid or doesn't have access
#   429              → key works but rate-limited (treated as OK for auth)
#   anything else    → see status code; could be regional/model permission
#
# Providers that have no free verification endpoint print SKIP with the
# reason. Don't add expensive probes here — credits are real money.
set -eu

probe() {
    name=$1
    method=$2
    url=$3
    auth_header=$4
    code=$(curl -s -o /dev/null -w "%{http_code}" -X "$method" -H "$auth_header" --max-time 10 "$url" 2>/dev/null || echo "000")
    case "$code" in
        2*)  verdict="OK" ;;
        401) verdict="UNAUTHORIZED — key invalid or unauthorized" ;;
        403) verdict="FORBIDDEN — key valid but no access to this resource" ;;
        404) verdict="NOT FOUND — endpoint may have changed" ;;
        429) verdict="RATE-LIMITED — key works (auth passed)" ;;
        000) verdict="NETWORK ERROR — connection failed or timed out" ;;
        5*)  verdict="UPSTREAM $code — provider issue, not your key" ;;
        *)   verdict="HTTP $code — investigate" ;;
    esac
    printf "%-12s  %-3s  %s\n" "$name" "$code" "$verdict"
}

probe_query() {
    name=$1
    url=$2
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$url" 2>/dev/null || echo "000")
    case "$code" in
        2*)  verdict="OK" ;;
        401) verdict="UNAUTHORIZED — key invalid" ;;
        403) verdict="FORBIDDEN — no access" ;;
        429) verdict="RATE-LIMITED — auth OK" ;;
        000) verdict="NETWORK ERROR" ;;
        *)   verdict="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "$name" "$code" "$verdict"
}

echo "Provider     Code  Verdict"
echo "------------ ----  -------"

[ -n "${MG_OPENAI_API_KEY:-}" ]      && probe "openai"      "GET" "https://api.openai.com/v1/models"                "Authorization: Bearer $MG_OPENAI_API_KEY"
[ -n "${MG_ANTHROPIC_API_KEY:-}" ]   && probe "anthropic"   "GET" "https://api.anthropic.com/v1/models"             "x-api-key: $MG_ANTHROPIC_API_KEY" \
                                                                                                                    -H "anthropic-version: 2023-06-01"
# Anthropic needs the version header — handled below if first call returns 400
if [ -n "${MG_ANTHROPIC_API_KEY:-}" ]; then
    code=$(curl -s -o /dev/null -w "%{http_code}" -H "x-api-key: $MG_ANTHROPIC_API_KEY" -H "anthropic-version: 2023-06-01" --max-time 10 "https://api.anthropic.com/v1/models" 2>/dev/null || echo "000")
    case "$code" in
        2*)  v="OK" ;;
        401) v="UNAUTHORIZED" ;;
        429) v="RATE-LIMITED — auth OK" ;;
        *)   v="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "anthropic" "$code" "$v"
fi

if [ -n "${MG_GEMINI_API_KEY:-}" ]; then
    # Gemini takes the key as a query param.
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "https://generativelanguage.googleapis.com/v1beta/models?key=$MG_GEMINI_API_KEY" 2>/dev/null || echo "000")
    case "$code" in
        2*)  v="OK" ;;
        400|401|403) v="UNAUTHORIZED" ;;
        429) v="RATE-LIMITED — auth OK" ;;
        *)   v="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "gemini" "$code" "$v"
fi

[ -n "${MG_XAI_API_KEY:-}" ]         && probe "xai"         "GET" "https://api.x.ai/v1/models"                      "Authorization: Bearer $MG_XAI_API_KEY"
[ -n "${MG_MISTRAL_API_KEY:-}" ]     && probe "mistral"     "GET" "https://api.mistral.ai/v1/models"                "Authorization: Bearer $MG_MISTRAL_API_KEY"
[ -n "${MG_DEEPSEEK_API_KEY:-}" ]    && probe "deepseek"    "GET" "https://api.deepseek.com/v1/models"              "Authorization: Bearer $MG_DEEPSEEK_API_KEY"
[ -n "${MG_PERPLEXITY_API_KEY:-}" ]  && probe "perplexity"  "GET" "https://api.perplexity.ai/models"                "Authorization: Bearer $MG_PERPLEXITY_API_KEY"

[ -n "${MG_ELEVENLABS_API_KEY:-}" ]  && probe "elevenlabs"  "GET" "https://api.elevenlabs.io/v1/voices"              "xi-api-key: $MG_ELEVENLABS_API_KEY"
[ -n "${MG_HUME_API_KEY:-}" ]        && probe "hume"        "GET" "https://api.hume.ai/v0/tts/voices?provider=HUME_AI" "X-Hume-Api-Key: $MG_HUME_API_KEY"

# Runway: list tasks (read-only).
[ -n "${MG_RUNWAY_API_KEY:-}" ] && {
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -H "Authorization: Bearer $MG_RUNWAY_API_KEY" \
        -H "X-Runway-Version: 2024-11-06" \
        "https://api.dev.runwayml.com/v1/tasks?limit=1" 2>/dev/null || echo "000")
    case "$code" in
        2*)  v="OK" ;;
        401) v="UNAUTHORIZED" ;;
        404) v="NOT FOUND — endpoint path or version may have changed" ;;
        429) v="RATE-LIMITED — auth OK" ;;
        *)   v="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "runway" "$code" "$v"
}

# Luma: list generations (read-only).
[ -n "${MG_LUMA_API_KEY:-}" ] && {
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -H "Authorization: Bearer $MG_LUMA_API_KEY" \
        "https://api.lumalabs.ai/dream-machine/v1/generations?limit=1" 2>/dev/null || echo "000")
    case "$code" in
        2*)  v="OK" ;;
        401) v="UNAUTHORIZED" ;;
        429) v="RATE-LIMITED — auth OK" ;;
        *)   v="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "luma" "$code" "$v"
}

# Pika: try a known doc'd info endpoint. The public API is gated behind a
# tier — many keys don't have access at all.
[ -n "${MG_PIKA_API_KEY:-}" ] && {
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -H "Authorization: Bearer $MG_PIKA_API_KEY" \
        "https://api.pika.art/v1/jobs?limit=1" 2>/dev/null || echo "000")
    case "$code" in
        2*)  v="OK" ;;
        401) v="UNAUTHORIZED — key invalid or no API tier" ;;
        404) v="NOT FOUND — your tier may not expose this endpoint" ;;
        429) v="RATE-LIMITED — auth OK" ;;
        *)   v="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "pika" "$code" "$v"
}

# Kling: auth is a JWT signed from access/secret pair, not a raw key.
# Without the signing step we can't probe — flag as SKIP.
[ -n "${MG_KLING_API_KEY:-}" ] && {
    printf "%-12s  %s\n" "kling" "SKIP — Kling needs a signed JWT bearer; raw key probe not meaningful"
}

# MiniMax: needs a region-routed endpoint.
[ -n "${MG_MINIMAX_API_KEY:-}" ] && {
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -H "Authorization: Bearer $MG_MINIMAX_API_KEY" \
        "https://api.minimax.chat/v1/files?limit=1" 2>/dev/null || echo "000")
    case "$code" in
        2*)  v="OK" ;;
        401) v="UNAUTHORIZED" ;;
        429) v="RATE-LIMITED — auth OK" ;;
        *)   v="HTTP $code" ;;
    esac
    printf "%-12s  %-3s  %s\n" "minimax" "$code" "$v"
}

# ByteDance Seedance: Volcano Ark, region-routed, JWT-signed. Skip raw probe.
[ -n "${MG_BYTEDANCE_API_KEY:-}" ] && {
    printf "%-12s  %s\n" "bytedance" "SKIP — Volcano Ark uses signed credentials; raw key probe not meaningful"
}

# Azure: requires a deployment-scoped endpoint we don't know here.
[ -n "${MG_AZURE_OPENAI_API_KEY:-}" ] && {
    printf "%-12s  %s\n" "azure" "SKIP — Azure needs an endpoint URL to probe; configure it in the UI to test"
}

echo
echo "Done."
