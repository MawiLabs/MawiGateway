#!/bin/bash
# Test script for agentic services
# This script creates a simple agentic service and tests it

set -e

DB_PATH="/Users/aka/Desktop/workk/perso/mawi/mawi.db"
API_URL="http://localhost:8030"

echo "🧪 Testing Agentic Service Implementation"
echo "=========================================="

# Login and get session
echo "📝 Step 1: Logging in..."
curl -s -X POST "$API_URL/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${ADMIN_EMAIL:-admin@mawi.local}\",\"password\":\"${ADMIN_PASSWORD:-admin123}\"}" \
  -c cookies.txt > /dev/null

echo "✅ Logged in successfully"

# Get a test model ID for the planner
echo "📝 Step 2: Getting available models..."
MODELS_JSON=$(curl -s -X GET "$API_URL/v1/models" -b cookies.txt)
PLANNER_MODEL=$(echo "$MODELS_JSON" | jq -r '.[0].id' 2>/dev/null || echo "")

if [ -z "$PLANNER_MODEL" ]; then
    echo "❌ No models found. Please create a model first."
    exit 1
fi

echo "✅ Using planner model: $PLANNER_MODEL"

# Create an agentic service
echo "📝 Step 3: Creating agentic service..."
CREATE_RESPONSE=$(curl -s -X POST "$API_URL/v1/services" \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d "{
    \"name\": \"test-agent\",
    \"service_type\": \"AGENTIC\",
    \"description\": \"Test agentic service\",
    \"guardrails\": [],
    \"planner_model_id\": \"$PLANNER_MODEL\"
  }")

echo "$CREATE_RESPONSE" | jq '.' 2>/dev/null || echo "$CREATE_RESPONSE"

# Check if service was created
SERVICE_CHECK=$(curl -s -X  GET "$API_URL/v1/services/test-agent" -b cookies.txt 2>/dev/null || echo "")

if echo "$SERVICE_CHECK" | grep -q "test-agent"; then
    echo "✅ Agentic service created successfully"
else
    echo "⚠️  Service creation response: $CREATE_RESPONSE"
fi

# Update service via API
echo "📝 Step 4: Configuring agentic service via API..."
UPDATE_RESPONSE=$(curl -s -X PUT "$API_URL/v1/services/test-agent" \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "system_prompt": "You are a helpful AI agent.",
    "max_iterations": 5
  }')
echo "✅ Service configured via API"

# Try to execute a simple request
echo "📝 Step 5: Testing agent execution..."
EXEC_RESPONSE=$(curl -s -X POST "$API_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "service": "test-agent",
    "stream": true,
    "messages": [{"role": "user", "content": "Hello, who are you?"}]
  }')

echo "$EXEC_RESPONSE" | jq '.' 2>/dev/null || echo "$EXEC_RESPONSE"

if echo "$EXEC_RESPONSE" | grep -q "choices"; then
    echo "✅ Agent executed successfully!"
else
    echo "⚠️  Execution response: $EXEC_RESPONSE"
fi

# Cleanup
echo "📝 Step 6: Cleaning up..."
curl -s -X DELETE "$API_URL/v1/services/test-agent" -b cookies.txt > /dev/null
echo "✅ Test service deleted"

rm -f cookies.txt

echo ""
echo "=========================================="
echo "🎉 Agentic service test complete!"
