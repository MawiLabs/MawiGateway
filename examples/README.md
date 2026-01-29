# MaWi Gateway Examples

This directory contains example scripts to test and demonstrate the MaWi Gateway functionality.

## Available Examples

### Basic Chat Test
```bash
./test_chat.sh
```
Tests basic chat completions (streaming and non-streaming).

### Phase 1 Integration Test
```bash
./test_phase1.sh
```
Comprehensive test of real provider integration. Checks:
- Backend connectivity
- Configured providers and services
- Service-to-model mappings
- Real AI API calls

### Agentic Workflow Test
```bash
./test_agentic.sh
```
Tests agentic execution with tool calling and multi-step reasoning.

### Backend Integration Tests
```bash
./test_backend.sh
```
Full backend API test suite.

```bash
./test_all.sh
```
Runs all tests sequentially.

## Prerequisites

- MaWi Gateway running on `http://localhost:8030`
- `curl` and `jq` installed
- Valid API keys configured (for real provider tests)

## Usage

All scripts can be run directly from this directory:
```bash
cd examples
./test_chat.sh
```
