# Audio Transcriber Integration Tests

This directory contains container-level integration tests for the `audio_transcriber` service. These tests verify that the audio transcriber works correctly when built as a container and interacting with AWS services.

## Overview

The integration tests use:
- **Testcontainers**: To manage Docker containers during testing
- **LocalStack**: To mock AWS services (S3, DynamoDB) locally
- **Docker Buildx**: To build the actual production container
- **Realistic test data**: WAV audio files and DynamoDB structures that match production

## Test Structure

### `integration_test.rs`

Comprehensive integration test with:
- Configurable timeouts and parameters
- Better error handling and debugging output
- Realistic test data and scenarios
- Cleanup and debugging options

### `test_config.rs`

Configuration utilities for customizing test behavior via environment variables.

## Running the Tests

### Prerequisites

1. **Docker**: Must be available and running
2. **Rust**: Development environment set up
3. **Network access**: For downloading container dependencies

### Basic Test Run

```bash
# Run all integration tests
cd audio_transcriber
cargo test --test integration_test
```

### Environment Variables

You can customize test behavior using environment variables:

```bash
# Set custom timeouts (in seconds)
export TEST_LOCALSTACK_TIMEOUT=60    # LocalStack startup timeout
export TEST_BUILD_TIMEOUT=1200       # Container build timeout (20 min)
export TEST_RUN_TIMEOUT=600          # Container run timeout (10 min)

# Customize test AWS resources
export TEST_BUCKET=my-test-bucket
export TEST_TABLE=my-test-table

# Debugging options
export TEST_CLEANUP=false            # Don't cleanup resources after test
export TEST_KEEP_CONTAINERS=true     # Keep containers running for debugging

# Run with custom settings
cargo test --test integration_test
```

### Expected Output

A successful test run will show:
```
🚀 Starting audio_transcriber integration test
📋 Test configuration: TestConfig { ... }
🐳 Starting LocalStack container...
✅ LocalStack started on port 4566
🗄️ Setting up test infrastructure...
✅ Created S3 bucket: test-input-bucket
✅ Uploaded test audio file: test-audio-1692547200.wav
✅ Created DynamoDB table: test-table
✅ Inserted test data into DynamoDB
🔨 Building audio_transcriber container...
✅ Container built successfully
🏃 Running audio_transcriber container...
✅ Container completed successfully
🔍 Verifying transcription results...
📝 Transcription result structure: M({"segments": L([...])})
✅ Found 1 transcription segments
✅ Transcription segment structure is valid
✅ All transcription verification checks passed!
🎉 Integration test completed successfully!
```

## What the Tests Verify

### Infrastructure Setup
- ✅ LocalStack container starts successfully
- ✅ S3 bucket creation and file upload
- ✅ DynamoDB table creation and data insertion

### Container Operations
- ✅ Audio transcriber container builds without errors
- ✅ Container runs with proper environment variables
- ✅ Container can connect to LocalStack services
- ✅ Container processes audio files correctly

### Data Processing
- ✅ Container reads silence segments from DynamoDB
- ✅ Container downloads audio files from S3
- ✅ Container generates transcription using Whisper
- ✅ Container writes transcription results back to DynamoDB

### Output Validation
- ✅ Transcription structure matches expected format
- ✅ Required fields are present in transcription
- ✅ Container logs show expected processing steps

## Debugging Failed Tests

### Container Build Failures
If the container build fails:
1. Check Docker is running: `docker version`
2. Verify you're in the workspace root when running tests
3. Check disk space for container builds
4. Review build logs in test output

### Container Runtime Failures
If the container runs but fails:
1. Set `TEST_KEEP_CONTAINERS=true` to inspect containers
2. Check LocalStack logs: `docker logs <localstack_container>`
3. Verify network connectivity between containers
4. Check AWS environment variables are set correctly

### LocalStack Issues
If LocalStack fails to start:
1. Ensure no other services are using port 4566
2. Check Docker has sufficient resources allocated
3. Increase `TEST_LOCALSTACK_TIMEOUT` for slower systems

### Performance Tuning
For slower systems, increase timeouts:
```bash
export TEST_BUILD_TIMEOUT=1800     # 30 minutes
export TEST_RUN_TIMEOUT=900        # 15 minutes
export TEST_LOCALSTACK_TIMEOUT=90  # 90 seconds
```

## Integration with CI/CD

These tests are designed to run in CI/CD environments:

```yaml
# Example GitHub Actions step
- name: Run Integration Tests
  run: |
    cd audio_transcriber
    cargo test --test integration_test
  env:
    TEST_BUILD_TIMEOUT: 1800
    TEST_RUN_TIMEOUT: 600
    TEST_CLEANUP: true
```

## Adding New Tests

To add new integration test scenarios:

1. Create a new test function in `integration_test.rs`
2. Use the `TestConfig` for consistent configuration
3. Follow the pattern: Setup → Build → Run → Verify → Cleanup
4. Add specific assertions for your use case
5. Document expected behavior in test comments

## Limitations

- **Network Dependencies**: Tests require internet access for container builds
- **Docker Requirements**: Tests need Docker with buildx support
- **Resource Usage**: Container builds can be resource-intensive
- **Platform Specific**: Some Docker operations may behave differently on different platforms

## Troubleshooting

### Common Issues

**"Failed to start LocalStack container"**
- Check Docker is running and accessible
- Verify no port conflicts on 4566
- Try increasing startup timeout

**"Container build timed out"**
- Increase `TEST_BUILD_TIMEOUT`
- Check available disk space
- Verify internet connectivity for downloading dependencies

**"Container failed with exit code 1"**
- Check container logs in test output
- Verify AWS environment variables
- Ensure LocalStack services are ready

**"Transcription field not found in DynamoDB item"**
- Container may have failed silently
- Check LocalStack connectivity
- Verify audio file format is supported
