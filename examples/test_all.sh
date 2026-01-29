#!/bin/bash
set -e

echo "🚀 Starting Full System Test Suite"
echo "=================================="

# 1. Run Unit & Integration Tests (Rust)
echo ""
echo "📦 Phase 1: Running Rust Unit & Integration Tests..."
echo "--------------------------------------------------"
cargo test --workspace
echo "✅ Rust tests passed."

# 2. Setup for E2E Tests
# We need the server running. This script assumes the server is ALREADY running at localhost:8030
# checking if server is up
if ! curl -s http://localhost:8030/health > /dev/null; then
    echo ""
    echo "⚠️  WARNING: MaWi Gateway is not running at http://localhost:8030"
    echo "   Please start the server with 'cargo run --bin gateway' in a separate terminal"
    echo "   before running the E2E api tests."
    echo "   Skipping E2E scripts."
    exit 0
fi

# 3. Run API E2E Tests
echo ""
echo "🌐 Phase 2: Running API E2E Tests..."
echo "----------------------------------"
chmod +x test_backend.sh
./test_backend.sh

# 4. Run Agentic Service Tests
echo ""
echo "🤖 Phase 3: Running Agentic Service Tests..."
echo "------------------------------------------"
chmod +x test_agentic.sh
./test_agentic.sh

echo ""
echo "🎉 ALL TESTS PASSED SUCCESSFULLY!"
