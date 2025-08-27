# Embedding Service Integration Tests

This directory contains comprehensive integration tests for the embedding service that test the complete workflow from DynamoDB data to PostgreSQL vector embeddings.

## Overview

The integration tests verify:
1. Processing video clips with transcriptions from DynamoDB
2. Generating embeddings using OpenAI API (or mock)
3. Storing embeddings in PostgreSQL with pgvector extension
4. Database schema initialization and vector indexing

## Test Infrastructure

The tests support two modes:

### 1. Docker Compose Mode (Recommended)
- **PostgreSQL with pgvector**: Vector database for embeddings  
- **LocalStack**: Mock AWS services (DynamoDB, SecretsManager)
- **WireMock**: Mock OpenAI API server with realistic responses
- **Isolated networking**: All services run in dedicated Docker network

### 2. Testcontainers Mode (Legacy)
- Uses Rust testcontainers library
- May have issues with SSL certificates in CI environments
- Fallback option when Docker Compose is not available

## Quick Start

### Prerequisites

1. **Docker & Docker Compose**: Must be installed and running
2. **Rust workspace**: Must be built first:
   ```bash
   cargo build --workspace
   ```

### Running Tests

#### Option 1: Using Docker Compose (Recommended)
```bash
cd embedding_service/tests
./run_integration_tests.sh
```

#### Option 2: Using the workspace test runner
```bash
# From workspace root
./run_integration_tests.sh embedding_service --build
```

#### Option 3: Direct cargo test (testcontainers mode)
```bash
cargo test -p embedding_service --test integration_test -- --ignored --nocapture
```

## Test Configuration

### Environment Variables

#### Test Infrastructure
- `USE_REAL_OPENAI`: Use real OpenAI API instead of mock (default: false)
- `BUILD_IMAGE`: Build Docker image before testing (default: true)
- `CLEANUP`: Cleanup resources after test (default: true)
- `VERBOSE`: Enable verbose output (default: false)
- `COMPOSE_PROJECT_NAME`: Docker Compose project name (default: embedding-test)

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
- `OPENAI_API_KEY`: Real OpenAI API key (required if USE_REAL_OPENAI=true)

### Example Commands

**Basic test run:**
```bash
cd embedding_service/tests
./run_integration_tests.sh
```

**With real OpenAI API:**
```bash
USE_REAL_OPENAI=true OPENAI_API_KEY=sk-... ./run_integration_tests.sh
```

**Keep infrastructure for debugging:**
```bash
CLEANUP=false ./run_integration_tests.sh
```

**Verbose output:**
```bash
VERBOSE=true ./run_integration_tests.sh
```

**Skip image build (if already built):**
```bash
BUILD_IMAGE=false ./run_integration_tests.sh
```

## Test Architecture

### Test Flow

1. **Infrastructure Setup**
   - Start PostgreSQL with pgvector extension
   - Start LocalStack for AWS services
   - Start WireMock for OpenAI API simulation

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

The WireMock server provides realistic OpenAI API responses:
- **Endpoint**: `/v1/embeddings`
- **Response**: JSON with 1536-dimensional random vectors
- **Features**: Proper usage metrics, error simulation

Configuration is in `fixtures/wiremock/mappings/embeddings.json`.

### Test Data

The tests create:
- **DynamoDB items**: With transcription text, summaries, metadata
- **PostgreSQL tables**: `embeddings` table with vector columns
- **Vector indexes**: HNSW indexes for similarity search

## Troubleshooting

### Common Issues

#### Docker Build Failures
```
Failed to build embedding service test image
```
**Solutions:**
- Check Docker daemon is running
- Verify network connectivity
- Try: `docker system prune` to free space
- Increase Docker memory/CPU limits

#### SSL Certificate Errors (testcontainers mode)
```
SSL certificate problem: self-signed certificate in certificate chain
```
**Solutions:**
- Use Docker Compose mode instead: `./run_integration_tests.sh`
- Set `BUILD_IMAGE=false` if image already exists
- Configure corporate proxy/certificates if needed

#### Port Conflicts
```
Port already in use
```
**Solutions:**
- Change `COMPOSE_PROJECT_NAME` to avoid conflicts
- Run: `docker-compose down` to cleanup orphaned containers
- Check for other services using ports 4566, 5432, 8080

#### Test Timeouts
```
PostgreSQL startup timed out
```
**Solutions:**
- Increase timeouts: `TEST_POSTGRES_TIMEOUT=60`
- Check system resources (Docker needs adequate RAM)
- Verify no firewall blocking container communication

#### Missing Dependencies
```
cargo: command not found
```
**Solutions:**
- Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Build workspace: `cargo build --workspace`
- Install Docker Compose: [Official installation guide](https://docs.docker.com/compose/install/)

### Debugging Tips

#### Keep Infrastructure Running
```bash
CLEANUP=false ./run_integration_tests.sh
```
Then connect to services:
- **PostgreSQL**: `psql -h localhost -p <port> -U test_user test_embeddings`
- **LocalStack**: `aws --endpoint-url=http://localhost:<port> dynamodb list-tables`
- **WireMock**: `curl http://localhost:<port>/__admin/mappings`

#### Check Container Logs
```bash
# List running containers
docker-compose -f docker-compose.test.yml -p embedding-test ps

# View logs
docker-compose -f docker-compose.test.yml -p embedding-test logs postgres-test
docker-compose -f docker-compose.test.yml -p embedding-test logs localstack-test
```

#### Manual Service Testing
```bash
# Test OpenAI mock
curl -X POST http://localhost:<port>/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input": "test", "model": "text-embedding-3-small"}'

# Test PostgreSQL
docker exec -it <postgres-container> psql -U test_user test_embeddings -c "SELECT version();"
```

### Performance Optimization

#### Faster Builds
- Use `BUILD_IMAGE=false` after first successful build
- Pre-pull images: `docker-compose pull`
- Use Docker BuildKit: `DOCKER_BUILDKIT=1`

#### Resource Limits
```yaml
# Add to docker-compose.test.yml services
mem_limit: 512m
cpus: '0.5'
```

## Files Overview

```
tests/
├── run_integration_tests.sh      # Main test runner script
├── docker-compose.test.yml       # Docker Compose configuration
├── Dockerfile.test               # Optimized Dockerfile for testing
├── integration_test.rs           # Main integration test
├── test_config.rs               # Test configuration management
├── mod.rs                       # Test module definitions
├── fixtures/
│   └── wiremock/
│       └── mappings/
│           └── embeddings.json  # OpenAI API mock configuration
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
        run: ./embedding_service/tests/run_integration_tests.sh
        env:
          CLEANUP: true
          VERBOSE: true
```

### Local Development
```bash
# Pre-commit check
./embedding_service/tests/run_integration_tests.sh

# Development cycle with kept infrastructure
CLEANUP=false BUILD_IMAGE=false ./embedding_service/tests/run_integration_tests.sh
```

The integration tests provide comprehensive validation of the embedding service in a realistic environment while being reliable and maintainable.
