# Embedding Service Integration Tests

This directory contains integration tests for the embedding service that test the complete workflow from DynamoDB data to PostgreSQL vector embeddings.

## Overview

The integration tests verify:
1. Processing video clips with transcriptions from DynamoDB
2. Generating embeddings using OpenAI API (or mock)
3. Storing embeddings in PostgreSQL with pgvector extension
4. Database schema initialization and vector indexing

## Test Infrastructure

The tests use:
- **LocalStack**: Mock AWS services (DynamoDB, SecretsManager)
- **PostgreSQL with pgvector**: Vector database for embeddings
- **Docker**: Container execution environment
- **testcontainers**: Container lifecycle management

## Running Tests

### Prerequisites

1. Docker must be installed and running
2. Rust workspace must be built:
   ```bash
   cargo build --workspace
   ```

### Basic Test Execution

Run the integration test (ignored by default):
```bash
cargo test --test integration_test -- --ignored
```

### Environment Variables

Configure test behavior with environment variables:

#### Timeouts
- `TEST_LOCALSTACK_TIMEOUT`: LocalStack startup timeout (default: 30s)
- `TEST_POSTGRES_TIMEOUT`: PostgreSQL startup timeout (default: 30s)  
- `TEST_BUILD_TIMEOUT`: Container build timeout (default: 600s)
- `TEST_RUN_TIMEOUT`: Container run timeout (default: 300s)

#### Test Resources
- `TEST_BUCKET`: S3 bucket name (default: "test-input-bucket")
- `TEST_TABLE`: DynamoDB table name (default: "test-table")
- `TEST_DATABASE`: PostgreSQL database name (default: "test_embeddings")
- `TEST_POSTGRES_USER`: PostgreSQL username (default: "test_user")
- `TEST_POSTGRES_PASSWORD`: PostgreSQL password (default: "test_password")

#### OpenAI Configuration
- `OPENAI_API_KEY`: Real OpenAI API key (optional)
- `TEST_USE_MOCK_OPENAI`: Use mock OpenAI endpoint (default: true)

#### Debugging
- `TEST_CLEANUP`: Cleanup resources after test (default: true)
- `TEST_KEEP_CONTAINERS`: Keep containers running for debugging (default: false)
- `TEST_IMAGE_NAME`: Override Docker image name

### Example Commands

Run with real OpenAI API:
```bash
OPENAI_API_KEY=sk-... TEST_USE_MOCK_OPENAI=false cargo test --test integration_test -- --ignored
```

Keep containers for debugging:
```bash
TEST_KEEP_CONTAINERS=true cargo test --test integration_test -- --ignored
```

Increase timeouts for slower environments:
```bash
TEST_BUILD_TIMEOUT=1200 TEST_RUN_TIMEOUT=600 cargo test --test integration_test -- --ignored
```

## Test Data

The tests create:
- DynamoDB items with transcription data
- PostgreSQL embeddings table with vector indices
- Mock OpenAI endpoints (when enabled)

## Troubleshooting

### Container Build Issues
- Ensure Docker has sufficient resources
- Check network connectivity for dependency downloads
- Increase `TEST_BUILD_TIMEOUT` for slower builds

### Test Failures
- Check Docker daemon is running
- Verify no port conflicts (LocalStack uses 4566, PostgreSQL uses 5432)
- Review container logs for specific errors

### Mock OpenAI Issues
- Ensure mock server starts correctly
- Check embedding response format matches expected structure
- Verify network connectivity between containers

## Test Structure

- `integration_test.rs`: Main integration test implementation
- `test_config.rs`: Test configuration and environment handling
- `fixtures/`: Test data and mock server configurations (future)
