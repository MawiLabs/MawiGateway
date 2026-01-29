#!/bin/bash
# Phase 1 Test Script - Real Provider Integration
# This tests that the gateway actually calls AI provider APIs

set -e

echo "🧪 Testing Phase 1: Real Provider Integration"
echo "=============================================="
echo ""

# Backend URL
API_URL="http://localhost:8030"

echo "1️⃣  Check backend is running..."
if ! curl -s "$API_URL/v1/providers" > /dev/null; then
    echo "❌ Backend not running on $API_URL"
    exit 1
fi
echo "✅ Backend is running"
echo ""

echo "2️⃣  Check configured providers..."
curl -s "$API_URL/v1/providers" | jq -r '.[] | "  - \(.name) (\(.provider_type))"'
echo ""

echo "3️⃣  Check configured services..."
curl -s "$API_URL/v1/services" | jq -r '.[] | "  - \(.name) (\(.service_type))"'
echo ""

echo "4️⃣  Check service models..."
SERVICES=$(curl -s "$API_URL/v1/services" | jq -r '.[].name')
for service in $SERVICES; do
    echo "  Service: $service"
    curl -s "$API_URL/v1/services/$service/models" | jq -r '.[] | "    - \(.model_name) (position: \(.position), weight: \(.weight))"'
done
echo ""

echo "5️⃣  Test chat completion..."
echo "  Payload:"
cat << EOF | tee /tmp/test_chat_request.json
{
  "service": "chat-prod",
  "messages": [
    {
      "role": "user",
      "content": "Say 'Hello from MaWi Gateway!' and nothing else."
    }
  ],
  "params": {
    "temperature": 0.7,
    "max_tokens": 50
  }
}
EOF
echo ""

echo "  Making request..."
RESPONSE=$(curl -s -X POST "$API_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d @/tmp/test_chat_request.json)

echo "  Response:"
echo "$RESPONSE" | jq '.'
echo ""

# Check if response contains actual content (not mock)
CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content // empty')
if echo "$CONTENT" | grep -q "Mock response"; then
    echo "⚠️  Still returning mock response"
    echo "   This means either:"
    echo "   - No valid API key configured"
    echo "   - Provider API call failed"
    echo "   - Need to check backend logs"
elif [ -n "$CONTENT" ]; then
    echo "✅ Got real AI response!"
    echo "   Content: $CONTENT"
else
    echo "❌ No content in response"
fi
echo ""

echo "=============================================="
echo "💡 To use real providers:"
echo "   1. Add a real API key to a provider (via frontend)"
echo "   2. Assign a model to your service"
echo "   3. Run this test again"
echo ""
echo "Check backend logs for detailed execution info:"
echo "   tail -f backend/logs or check the terminal"
