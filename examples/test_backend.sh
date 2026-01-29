#!/bin/bash

# MaWi Backend Test Script

BASE_URL="http://localhost:8030"

echo "🧪 Testing MaWi AI Gateway Backend"
echo "=================================="
echo ""

# Test 1: Create Provider
echo "1️⃣ Creating OpenAI Provider..."
PROVIDER_RESPONSE=$(curl -s -X POST "$BASE_URL/v1/providers" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "OpenAI Production",
    "provider_type": "openai",
    "description": "Main OpenAI account"
  }')

PROVIDER_ID=$(echo $PROVIDER_RESPONSE | jq -r '.id')
echo "   ✅ Provider created: $PROVIDER_ID"
echo ""

# Test 2: List Providers
echo "2️⃣ Listing all providers..."
curl -s "$BASE_URL/v1/providers" | jq '.[] | {id, name, provider_type}'
echo ""

# Test 3: Create Model
echo "3️⃣ Creating GPT-4 Model..."
MODEL_RESPONSE=$(curl -s -X POST "$BASE_URL/v1/models" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"GPT-4 Turbo\",
    \"provider_id\": \"$PROVIDER_ID\",
    \"modality\": \"text\",
    \"description\": \"Latest GPT-4 model\"
  }")

MODEL_ID=$(echo $MODEL_RESPONSE | jq -r '.id')
echo "   ✅ Model created: $MODEL_ID"
echo ""

# Test 4: List Models
echo "4️⃣ Listing all models..."
curl -s "$BASE_URL/v1/models" | jq '.[] | {id, name, modality}'
echo ""

# Test 5: Create Service
echo "5️⃣ Creating Chat Service..."
curl -s -X POST "$BASE_URL/v1/services" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "customer-chat",
    "service_type": "chat",
    "description": "Customer support chat service",
    "strategy": "leader-with-failover",
    "guardrails": []
  }' | jq '.'
echo ""

# Test 6: List Services
echo "6️⃣ Listing all services..."
curl -s "$BASE_URL/v1/services" | jq '.[] | {name, service_type, strategy}'
echo ""

# Test 7: Assign Model to Service
echo "7️⃣ Assigning GPT-4 to customer-chat service..."
curl -s -X POST "$BASE_URL/v1/services/customer-chat/models" \
  -H "Content-Type: application/json" \
  -d "{
    \"model_id\": \"$MODEL_ID\",
    \"modality\": \"text\",
    \"position\": 0
  }" | jq '.'
echo ""

# Test 8: Chat Completion
echo "8️⃣ Testing chat completion..."
curl -s -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "service": "customer-chat",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "params": {
      "temperature": 0.7
    }
  }' | jq '.'
echo ""

echo "✅ All tests completed!"
echo ""
echo "📊 View Swagger UI: $BASE_URL/swagger-ui"
