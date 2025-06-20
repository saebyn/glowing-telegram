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

## Test Coverage

The tests are configured to require a minimum of 20% coverage and generate HTML reports in the `htmlcov/` directory.

## Test Structure

- `test_python_modules.py` - Main test file containing tests for Python components
- `pyproject.toml` - Pytest configuration
- Coverage focuses on:
  - `media_lambda/main.py` - AWS Lambda for playlist generation
  - `cdk/lib/websocketAuthorizer/main.py` - WebSocket authorization

## CI/CD

The GitHub Actions workflow (`.github/workflows/python.yml`) automatically runs tests on:
- Push to main branch
- Pull requests to main branch

Tests must pass before code can be merged.