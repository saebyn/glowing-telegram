# Embedding Service Integration Tests

This directory contains comprehensive integration tests for the embedding service that test the complete workflow from DynamoDB data to PostgreSQL vector embeddings.

## Overview

The integration tests verify:
1. Processing video clips with transcriptions from DynamoDB
2. Generating embeddings using OpenAI API (or mock)
3. Storing embeddings in PostgreSQL with pgvector extension
4. Database schema initialization and vector indexing

## Test Infrastructure

The tests use **testcontainers** to provide isolated test infrastructure:

- **PostgreSQL with pgvector**: Vector database for embeddings  
- **LocalStack**: Mock AWS services (DynamoDB, SecretsManager)
- **Mock OpenAI server**: Simple HTTP server for OpenAI API simulation

## Quick Start

### Prerequisites

1. **Docker**: Must be installed and running
2. **Rust workspace**: Must be built first:
   ```bash
   cargo build --workspace
   ```

### Running Tests

#### Option 1: Using the test runner script
```bash
cd embedding_service/tests
./run_integration_tests.sh
```

#### Option 2: Direct cargo test
```bash
cargo test -p embedding_service --test integration_test -- --ignored --nocapture
```

## Test Configuration

### Environment Variables

#### Test Resources
- `TEST_BUCKET`: S3 bucket name (default: "test-input-bucket")
- `TEST_TABLE`: DynamoDB table name (default: "test-table")
- `TEST_DATABASE`: PostgreSQL database name (default: "test_embeddings")
- `TEST_POSTGRES_USER`: PostgreSQL username (default: "test_user")
- `TEST_POSTGRES_PASSWORD`: PostgreSQL password (default: "test_password")

#### Timeouts
- `TEST_LOCALSTACK_TIMEOUT`: LocalStack startup timeout (default: 30s)
- `TEST_POSTGRES_TIMEOUT`: PostgreSQL startup timeout (default: 30s)  
- `TEST_BUILD_TIMEOUT`: Container build timeout (default: 600s)
- `TEST_RUN_TIMEOUT`: Container run timeout (default: 300s)

#### OpenAI Configuration
- `USE_MOCK_OPENAI`: Use mock OpenAI API instead of real (default: true)
- `OPENAI_API_KEY`: Real OpenAI API key (required if USE_MOCK_OPENAI=false)

### Example Commands

**Basic test run:**
```bash
cd embedding_service/tests
./run_integration_tests.sh
```

**With real OpenAI API:**
```bash
USE_MOCK_OPENAI=false OPENAI_API_KEY=sk-... cargo test -p embedding_service --test integration_test -- --ignored --nocapture
```

**Keep containers for debugging:**
```bash
TEST_KEEP_CONTAINERS=true cargo test -p embedding_service --test integration_test -- --ignored --nocapture
```

## Test Architecture

### Test Flow

1. **Infrastructure Setup**
   - Start PostgreSQL with pgvector extension
   - Start LocalStack for AWS services
   - Start mock OpenAI server (if enabled)

2. **Data Preparation**
   - Create DynamoDB table with test schema
   - Create AWS Secrets (database credentials, OpenAI key)
   - Insert test data with transcriptions and summaries

3. **Service Testing**
   - Run embedding service to process single video clip
   - Run embedding service to scan entire stream
   - Verify embeddings stored correctly in PostgreSQL

4. **Validation**
   - Check database schema and extensions
   - Verify embedding content and dimensions
   - Validate vector indexes and metadata

### Mock OpenAI API

The built-in mock server provides simple OpenAI API responses:
- **Endpoint**: `/v1/embeddings`
- **Response**: JSON with 1536-dimensional dummy vectors
- **Features**: Basic usage metrics simulation

## Troubleshooting

### Common Issues

#### Docker Build Failures
```
Failed to build embedding service image
```
**Solutions:**
- Check Docker daemon is running
- Verify network connectivity
- Try: `docker system prune` to free space
- Increase Docker memory/CPU limits

#### Test Timeouts
```
PostgreSQL startup timed out
```
**Solutions:**
- Increase timeouts: `TEST_POSTGRES_TIMEOUT=60s`
- Check system resources (Docker needs adequate RAM)
- Verify no firewall blocking container communication

#### Missing Dependencies
```
cargo: command not found
```
**Solutions:**
- Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Build workspace: `cargo build --workspace`

### Debugging Tips

#### Keep Infrastructure Running
```bash
TEST_KEEP_CONTAINERS=true cargo test -p embedding_service --test integration_test -- --ignored --nocapture
```
Then connect to services manually for debugging.

## Files Overview

```
tests/
├── run_integration_tests.sh      # Simple test runner script
├── integration_test.rs           # Main integration test
├── test_config.rs               # Test configuration management
├── mod.rs                       # Test module definitions
└── README.md                    # This file
```

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Integration Tests
on: [push, pull_request]

jobs:
  integration-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build workspace
        run: cargo build --workspace
      - name: Run integration tests
        run: cargo test -p embedding_service --test integration_test -- --ignored --nocapture
```

The integration tests provide comprehensive validation of the embedding service using testcontainers for reliable and isolated testing.
