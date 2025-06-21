# Python Testing

This repository includes Python testing infrastructure that runs automatically on pull requests.

## Running Tests Locally

1. Install test dependencies:
```bash
pip install pytest pytest-cov boto3 pyjwt cryptography psycopg2-binary
```

2. Run tests:
```bash
python -m pytest
```

3. Run tests with verbose output:
```bash
python -m pytest -v
```

## Test Coverage

The tests are configured to require a minimum of 20% coverage and generate HTML reports in the `htmlcov/` directory.

## Test Structure

Tests are co-located with the Python modules they test for better organization and maintainability:

- `media_lambda/test_media_lambda.py` - Tests for AWS Lambda playlist generation
- `cdk/lib/websocketAuthorizer/test_websocket_authorizer.py` - Tests for WebSocket authorization
- `scripts/test_pg2ddb.py` - Tests for database migration utilities
- `scripts/test_retrigger_all_videos.py` - Tests for video processing utilities
- `audio_transcriber/test_download_model.py` - Tests for Whisper model downloader

### Automatic Test Discovery

The test infrastructure automatically discovers:
- New Python modules added to any directory
- New test files following the `test_*.py` naming convention
- Test functions starting with `test_`

No configuration updates are needed when adding new Python files or tests.

### Python Components Tested

- `media_lambda/main.py` - AWS Lambda for M3U8 playlist generation
- `cdk/lib/websocketAuthorizer/main.py` - JWT-based WebSocket authorization
- `scripts/pg2ddb.py` - Database migration utilities
- `scripts/retrigger_all_videos.py` - Video processing utilities  
- `audio_transcriber/download_model.py` - Whisper model downloader

## CI/CD

The GitHub Actions workflow (`.github/workflows/python.yml`) automatically runs tests on:
- Push to main branch
- Pull requests to main branch

Tests must pass before code can be merged.