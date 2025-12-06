# Multi-Environment Deployment - Quick Start

This document provides a quick overview of the multi-environment deployment system. For complete details, see [Multi-Environment Setup Guide](multi-environment-setup.md).

## What's New

The glowing-telegram project now supports **multiple isolated deployment environments**:

- **Production**: Live production environment (existing, no changes required)
- **Staging**: Pre-production testing environment
- **Dev**: Development and feature testing environment

## Key Features

✅ **Separate Infrastructure**: Each environment has isolated AWS resources (stacks, databases, Lambda functions, etc.)  
✅ **Environment-Specific Naming**: Non-production environments use suffixed stack names (e.g., `AppStack-dev`)  
✅ **Flexible Deployment**: Deploy any branch to any environment via GitHub Actions  
✅ **Production-Safe**: Existing production deployment unchanged and fully backward compatible  
✅ **Easy Configuration**: Single JSON file defines all environments  
✅ **Automated Outputs**: Script extracts configuration for frontend repository setup

## Quick Start - Deploy to Dev

### 1. Deploy Backend Infrastructure

```bash
cd glowing-telegram/cdk

# Install dependencies
npm ci

# Build CDK
npm run build

# Deploy to dev environment
ENVIRONMENT=dev IMAGE_VERSION=latest npm run cdk deploy --all
```

### 2. Extract Configuration for Frontend

```bash
# Export dev environment outputs
./scripts/export-stack-outputs.sh dev
```

This creates `frontend-config/` directory with configuration values.

### 3. GitHub Environments (Automated!)

GitHub environments are **automatically created and configured** when you deploy the CDK stacks!

**Prerequisites**:
1. Create a GitHub Personal Access Token with `repo` scope
2. Store in Secrets Manager:
```bash
aws secretsmanager create-secret \
  --name glowing-telegram/github-token \
  --secret-string '{"token":"ghp_your_token_here"}'
```

The CDK will automatically:
- Create environments in both repos (glowing-telegram and glowing-telegram-frontend)
- Set all required variables (API_URL, AWS_REGION, BUCKET_NAME, etc.)
- Update variables on each deployment to stay in sync

**Manual Setup (Optional)**:
If you prefer manual setup or want to skip automation, set `SKIP_GITHUB_ENV=true` and follow the [GitHub Environment Setup Guide](github-environment-setup.md).

### 4. Deploy Frontend to Dev

In `glowing-telegram-frontend` repository:

1. Go to **Actions** → Find deployment workflow
2. Click **Run workflow**
3. Select environment: `dev`
4. Click **Run workflow**

Done! Your dev environment is now fully deployed and operational.

## Quick Start - Deploy Backend Branch to Environment

### Via GitHub Actions (Recommended)

1. Go to `glowing-telegram` repository → **Actions** → "Deploy to Environment"
2. Click **Run workflow**
3. Select:
   - Environment: `dev` or `staging`
   - Branch: Your feature branch (e.g., `feature/my-feature`)
   - Image tag: Leave empty (will build from branch)
4. Click **Run workflow**

The workflow will:
- Build Docker images from your branch
- Tag as `dev-feature-my-feature-20231215-120000`
- Deploy all infrastructure stacks

### Via Command Line

```bash
cd glowing-telegram

# Set environment and image version
export ENVIRONMENT=dev
export IMAGE_VERSION=dev-latest

# Build Docker images (optional, if images not already built)
docker buildx bake -f docker-bake.hcl -f docker-bake.override.hcl all

# Push to ECR (requires AWS credentials)
# ... push commands ...

# Deploy CDK stacks
cd cdk
npm run cdk deploy --all
```

## Production Deployment (Unchanged)

Production deployment continues to work exactly as before:

1. Create a GitHub release (e.g., `v1.2.3`)
2. Automated workflow builds and deploys to production
3. No manual intervention required

## Environment Configuration

Edit `cdk/config/environments.json` to:
- Add new environments
- Change AWS account/region
- Update default frontend versions
- Modify environment tags

Example:
```json
{
  "environments": {
    "production": { "awsAccount": "...", "awsRegion": "us-west-2", ... },
    "staging": { "awsAccount": "...", "awsRegion": "us-west-2", ... },
    "dev": { "awsAccount": "...", "awsRegion": "us-west-2", ... }
  },
  "default": "production"
}
```

## Architecture

### Stack Naming

| Environment | FrontendStack | RepoStack | AppStack |
|-------------|---------------|-----------|----------|
| Production  | FrontendStack | RepoStack | AppStack |
| Staging     | FrontendStack-staging | RepoStack-staging | AppStack-staging |
| Dev         | FrontendStack-dev | RepoStack-dev | AppStack-dev |

### Resource Naming

- **Production**: Uses base names (backward compatible)
- **Non-production**: Adds environment prefix/suffix
  - S3 Buckets: `glowing-telegram-frontend-dev`
  - IAM Roles: `GlowingTelegram-GithubActionRole-dev`
  - DynamoDB Tables: Use stack prefix (includes environment)

## Key Files

| File | Purpose |
|------|---------|
| `cdk/config/environments.json` | Environment definitions |
| `cdk/lib/util/environment.ts` | Environment utilities |
| `.github/workflows/deploy-environment.yml` | Manual environment deployment |
| `.github/workflows/deploy.yml` | Production release deployment |
| `scripts/export-stack-outputs.sh` | Extract stack configuration |

## Documentation

- **[Multi-Environment Setup Guide](multi-environment-setup.md)** - Complete setup and configuration
- **[GitHub Environment Setup](github-environment-setup.md)** - Frontend repository configuration
- **[Documentation Index](README.md)** - All documentation links

## Troubleshooting

### Stack Already Exists
```bash
# Check if stack exists
aws cloudformation describe-stacks --stack-name AppStack-dev

# If exists, update instead:
ENVIRONMENT=dev npm run cdk deploy AppStack-dev
```

### Can't Assume IAM Role
- Verify AWS credentials are configured
- Check IAM role exists in AWS Console
- Ensure OIDC provider trust policy is correct

### Frontend Can't Connect to API
- Run `./scripts/export-stack-outputs.sh dev`
- Verify API_URL is correct in GitHub Environment
- Check API Gateway exists and is deployed

### CloudFormation Export Conflicts
- Exports are only created for production environment
- Non-production uses outputs without exports
- Use `aws cloudformation describe-stacks` to get values

## Testing

Run CDK tests:
```bash
cd cdk
npm test
```

All 11 environment configuration tests should pass.

## Security

- Each environment has isolated IAM roles and security groups
- Secrets Manager secrets are environment-specific
- CloudFormation exports only for production (avoid conflicts)
- Stack outputs available via API for non-production

## Next Steps

1. Deploy dev environment following Quick Start above
2. Test deployment workflow via GitHub Actions
3. Deploy staging environment for QA testing
4. Configure third-party services (Twitch, YouTube, OpenAI)
5. Review [complete documentation](multi-environment-setup.md) for advanced features

## Support

- Check documentation: `docs/multi-environment-setup.md`
- Review workflow logs in GitHub Actions
- Check CloudWatch logs for runtime errors
- Verify AWS resources in CloudFormation console

---

**Note**: This feature is production-ready and backward compatible. Existing production infrastructure is unchanged and continues to work without modification.
