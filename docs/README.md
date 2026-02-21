# Glowing Telegram Documentation

This directory contains documentation for the glowing-telegram project.

## Architecture and Design

- **ER Diagrams**: Database entity-relationship diagrams in `schemas/` directory
- **JSON Schemas**: Type definitions in `v2/schemas/` directory
- **Workflow Diagrams**: Process flow documentation

### Deployment and Operations

- **[Initial Setup Guide](INITIAL_SETUP.md)**: First-time setup for deploying to dev and production environments
- **[Normal Operation Guide](NORMAL_OPERATION.md)**: Day-to-day deployment workflows via GitHub Actions
  - Environment configuration
  - Deployment workflows
  - GitHub Actions setup
  - Third-party service integration
  - Troubleshooting and best practices
- **[GitHub Environment Automation](github-environment-automation.md)**: Automated GitHub environment management (NEW!)
  - Automatic creation/update of GitHub environments
  - Automated variable synchronization
  - GitHub API integration via CDK custom resource
  - Supports both backend and frontend repositories

## Configuration Files

### Environment Configuration

The project supports multiple deployment environments configured in `cdk/config/environments.json`:

- **Production**: Live production environment (default)
- **Staging**: Pre-production testing environment
- **Dev**: Development and feature testing environment

Each environment has isolated AWS infrastructure including:
- Separate CloudFormation stacks
- Isolated S3 buckets and DynamoDB tables
- Environment-specific Lambda functions
- Separate IAM roles and security groups

### Frontend Configuration

The `v2/schemas/` directory contains JSON schemas that define:
- API request/response formats
- Data models for streams, episodes, and projects
- Widget configurations
- WebSocket message formats

These schemas are used to generate TypeScript and Rust type definitions via the `types/import.sh` script.

## Quick Links

- [Main README](../README.md) - Project overview and getting started
- [Multi-Environment Setup](multi-environment-setup.md) - Deployment guide
- [CDK Documentation](../cdk/README.md) - Infrastructure as Code details
- [Type Generation Script](../types/import.sh) - Generate types from schemas

## Development Resources

### Deployment Workflows

Located in `.github/workflows/`:
- `deploy.yml` - Automated production deployment via releases
- `deploy-environment.yml` - Manual deployment to dev/staging
- `cdk-deploy-reusable.yml` - Reusable CDK deployment workflow
- `docker-build-reusable.yml` - Reusable Docker build workflow

### Deployment Scripts

Located in `scripts/`:
- `export-stack-outputs.sh` - Extract CDK stack outputs for frontend configuration
- `push_image.sh` - Manually push Docker images to ECR
- `push_all.sh` - Push all Docker images to ECR

### Configuration Management

Environment-specific configuration:
- `cdk/config/environments.json` - Environment definitions
- `cdk/config/version.json` - Frontend version tracking
- `cdk/lib/util/environment.ts` - Environment utility functions

## Testing

### CDK Tests

Run CDK infrastructure tests:
```bash
cd cdk
npm test
```

Key test files:
- `test/environment.test.ts` - Environment configuration tests
- `test/frontendStack.test.ts` - Frontend stack tests
- `test/gt-cdk.test.ts` - Core infrastructure tests

### Integration Tests

Run service integration tests:
```bash
./run_integration_tests.sh <service_name>
```

Examples:
```bash
./run_integration_tests.sh audio_transcriber
./run_integration_tests.sh video_ingestor
./run_integration_tests.sh embedding_service
```

## Contributing

When adding new features or services:

1. Update relevant JSON schemas in `v2/schemas/`
2. Run `./types/import.sh` to regenerate type definitions
3. Update CDK infrastructure in `cdk/lib/`
4. Add tests in `cdk/test/`
5. Update documentation in this directory
6. Test in dev environment before promoting to staging/production

See the [Multi-Environment Setup Guide](multi-environment-setup.md) for deployment best practices.
