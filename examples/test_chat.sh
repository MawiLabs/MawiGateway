#!/bin/bash

echo "Testing MaWi AI Gateway"
echo "======================="
echo

# Test 1: Non-streaming chat
echo "1. Non-streaming chat:"
curl -s http://localhost:8030/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }' | jq '.'

echo
echo

# Test 2: Streaming chat
echo "2. Streaming chat (SSE):"
curl -s http://localhost:8030/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'

echo
echo
echo "✅ Tests complete"
