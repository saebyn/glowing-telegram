# Multi-Environment Deployment Setup

This document describes how to set up and deploy to multiple environments (dev, staging, production) in the glowing-telegram system.

## Overview

The system supports three environments:

- **Production**: The main production environment for live users
- **Staging**: Pre-production environment for testing releases before deploying to production
- **Dev**: Development environment for testing features and changes

Each environment has its own isolated infrastructure including:
- Separate CloudFormation stacks with environment suffixes (e.g., `AppStack-dev`, `AppStack-staging`)
- Separate S3 buckets for frontend assets
- Separate DynamoDB tables, Lambda functions, and other AWS resources
- Separate ECR repositories and Docker image tags
- Environment-specific IAM roles for GitHub Actions

## Environment Configuration

Environments are defined in `cdk/config/environments.json`. Each environment specifies:

- **awsAccount**: AWS account ID where resources are deployed
- **awsRegion**: AWS region for deployment
- **frontendVersion**: Default frontend version to use
- **tags**: AWS tags applied to all resources in the environment

### Example Configuration

```json
{
  "environments": {
    "production": {
      "description": "Production environment",
      "awsAccount": "159222827421",
      "awsRegion": "us-west-2",
      "frontendVersion": "0.4.1",
      "tags": {
        "Environment": "production",
        "ManagedBy": "CDK"
      }
    },
    "staging": {
      "description": "Staging environment for testing before production",
      "awsAccount": "159222827421",
      "awsRegion": "us-west-2",
      "frontendVersion": "latest",
      "tags": {
        "Environment": "staging",
        "ManagedBy": "CDK"
      }
    }
  },
  "default": "production"
}
```

## Deployment Methods

### 1. Deploy via GitHub Actions (Recommended)

#### Deploy to Production (Automated Release)

Production deployments are automated when you create a GitHub release:

1. Go to GitHub repository → Releases → Create a new release
2. Create a new tag (e.g., `v1.2.3`)
3. Publish the release
4. GitHub Actions automatically:
   - Builds Docker images with the release tag
   - Deploys to production environment

**Workflow**: `.github/workflows/deploy.yml`

#### Deploy to Dev/Staging (Manual Workflow)

To deploy a specific branch to dev or staging:

1. Go to GitHub repository → Actions → "Deploy to Environment"
2. Click "Run workflow"
3. Select:
   - **Environment**: `dev` or `staging`
   - **Branch**: The branch to deploy (e.g., `feature/my-feature`, `main`)
   - **Image tag** (optional): Existing Docker image tag, or leave empty to build from branch
4. Click "Run workflow"

**Workflow**: `.github/workflows/deploy-environment.yml`

The workflow will:
- Build Docker images from the specified branch (if no image tag provided)
- Deploy infrastructure stacks (FrontendStack, RepoStack, AppStack) for the target environment
- Use environment-specific stack names (e.g., `AppStack-dev`)

### 2. Deploy Manually via CDK

For local development or testing:

```bash
cd cdk

# Install dependencies
npm ci

# Build CDK code
npm run build

# Deploy to dev environment
ENVIRONMENT=dev IMAGE_VERSION=latest npm run cdk deploy --all

# Deploy to staging environment
ENVIRONMENT=staging IMAGE_VERSION=v1.2.3 npm run cdk deploy --all

# Deploy to production environment (default)
IMAGE_VERSION=v1.2.3 npm run cdk deploy --all
```

## Setting Up a New Environment

### 1. Add Environment Configuration

Edit `cdk/config/environments.json` to add a new environment:

```json
{
  "environments": {
    "production": { ... },
    "staging": { ... },
    "dev": { ... },
    "qa": {
      "description": "QA environment for testing",
      "awsAccount": "159222827421",
      "awsRegion": "us-west-2",
      "frontendVersion": "latest",
      "tags": {
        "Environment": "qa",
        "ManagedBy": "CDK"
      }
    }
  },
  "default": "production"
}
```

### 2. Update GitHub Workflow

Add the new environment to the workflow choice options in `.github/workflows/deploy-environment.yml`:

```yaml
environment:
  description: 'Target environment to deploy to'
  required: true
  type: choice
  options:
    - dev
    - staging
    - qa        # Add new environment
    - production
```

### 3. Deploy Infrastructure

Deploy the new environment:

```bash
ENVIRONMENT=qa IMAGE_VERSION=latest npm run cdk deploy --all
```

Or use GitHub Actions workflow to deploy.

### 4. Configure AWS Resources

After initial deployment, you may need to manually configure:

#### a. Secrets Manager

Create or populate secrets for the environment. Secrets are automatically created by CDK with names based on the stack:

- **OpenAI API Key**: Created as `AppStack-OpenAISecret-*` (production) or `AppStack-{env}-OpenAISecret-*` (non-production)
  - Manually populate the secret value in AWS Secrets Manager Console
  - Example: Set the secret value to your OpenAI API key
- **YouTube App Credentials**: Created as `AppStack-YoutubeAppSecret-*` (production) or `AppStack-{env}-YoutubeAppSecret-*` (non-production)
  - Manually populate with YouTube OAuth client credentials from Google Cloud Console
  - Format: JSON with `client_id`, `client_secret`, and `redirect_uris`
- **Twitch EventSub Secret**: Configure in Twitch Developer Console
  - Update the `EVENTSUB_SECRET` environment variable in twitch_lambda
  - Must match the secret configured in Twitch EventSub webhook subscription

#### b. Cognito User Pool

The CDK creates a Cognito User Pool for authentication. You'll need to:

1. Create initial admin users in AWS Cognito Console
2. Configure application clients if needed
3. Note the User Pool ID and Client ID for frontend configuration

#### c. Aurora PostgreSQL Database

The CDK creates an Aurora Serverless v2 cluster with pgvector extension. Initial setup:

1. The database is automatically initialized with pgvector extension
2. Connection details are stored in Secrets Manager
3. The embedding service will automatically create required tables on first run

### 5. Configure Frontend (glowing-telegram-frontend)

The frontend repository needs environment-specific configuration exported from CDK stacks.

#### Export CDK Outputs

CDK stacks output important configuration values. To make these available to the frontend:

```bash
cd cdk
aws cloudformation describe-stacks \
  --stack-name AppStack-dev \
  --query 'Stacks[0].Outputs' \
  --output json > ../frontend-config-dev.json
```

**Key Outputs to Configure in Frontend**:

- **API Gateway URL**: REST API endpoint for CRUD operations
- **WebSocket API URL**: WebSocket endpoint for real-time features
- **CloudFront Distribution**: Frontend asset distribution domain
- **Cognito User Pool ID**: For authentication
- **Cognito User Pool Client ID**: For authentication
- **S3 Bucket Name**: For frontend asset uploads

#### GitHub Environment Settings

Create a GitHub environment in the `glowing-telegram-frontend` repository:

1. Go to frontend repository → Settings → Environments
2. Create environment (e.g., `dev`, `staging`)
3. Configure environment secrets and variables:

**Secrets**:
- `AWS_ROLE_ARN`: IAM role for deploying frontend assets (from RepoStack output)
- `FRONTEND_BUCKET`: S3 bucket name (from FrontendStack output)

**Variables**:
- `API_URL`: API Gateway URL (from AppStack output)
- `WEBSOCKET_URL`: WebSocket API URL (from AppStack output)
- `USER_POOL_ID`: Cognito User Pool ID (from AppStack output)
- `USER_POOL_CLIENT_ID`: Cognito Client ID (from AppStack output)

### 6. Third-Party Service Configuration

#### Twitch Integration

For Twitch chat integration and EventSub webhooks:

1. Register application at https://dev.twitch.tv/console
2. Configure redirect URLs for your environment's domain
3. Set up EventSub webhook URL pointing to your API Gateway endpoint
4. Store Twitch app credentials in AWS Secrets Manager
5. Update `EVENTSUB_SECRET` environment variable in twitch_lambda

#### YouTube Integration

For YouTube upload functionality:

1. Create OAuth2 credentials in Google Cloud Console
2. Store credentials in AWS Secrets Manager at `gt/youtube/user/<user_id>`
3. Configure OAuth redirect URLs for your environment
4. Test upload functionality with a sample video

#### OpenAI Integration

For AI features (transcription summaries, embeddings, chat):

1. Obtain API key from https://platform.openai.com/
2. Store in AWS Secrets Manager (OpenAISecret created by CDK)
3. Monitor usage and set billing limits as needed

## Stack Naming Convention

Stacks use environment-specific names:

- **Production**: Uses base names without suffix (`AppStack`, `FrontendStack`, `RepoStack`)
- **Non-production**: Adds environment suffix (`AppStack-dev`, `FrontendStack-staging`)

This ensures production maintains backward compatibility while allowing isolated environments.

## Resource Naming Convention

Resources within stacks use environment-specific names where appropriate:

- **Production**: Uses base names without prefix
- **Non-production**: Adds environment prefix for key resources

Examples:
- S3 Buckets: `glowing-telegram-frontend-dev`, `glowing-telegram-frontend-staging`
- IAM Roles: `GlowingTelegram-GithubActionRole-dev`
- DynamoDB Tables: Include stack prefix which contains environment

## Docker Image Tagging

Docker images are tagged based on deployment type:

- **Production releases**: Use version tags from GitHub releases (e.g., `v1.2.3`)
- **Environment deployments**: Use format `{env}-{branch}-{timestamp}` (e.g., `dev-feature-auth-20231201-143022`)

All images are stored in the same ECR repositories, differentiated by tags.

## Cleanup and Cost Management

### Removing an Environment

To remove a complete environment, destroy stacks in the correct dependency order (application stack first, then infrastructure stacks):

```bash
cd cdk

# Destroy stacks in correct dependency order
# AppStack first (depends on RepoStack and FrontendStack)
ENVIRONMENT=dev npm run cdk destroy AppStack-dev

# RepoStack next (depends on FrontendStack)
ENVIRONMENT=dev npm run cdk destroy RepoStack-dev

# FrontendStack last (no dependencies)
ENVIRONMENT=dev npm run cdk destroy FrontendStack-dev
```

**Warning**: This will delete all resources including data in DynamoDB tables and S3 buckets (if RemovalPolicy allows).

### Cost Optimization Tips

1. **Aurora Serverless**: Configure appropriate min/max ACU for non-production environments
2. **EFS**: Delete unused model caches periodically
3. **CloudWatch Logs**: Set appropriate retention periods (1 week default)
4. **S3 Versioning**: Disable for dev/staging buckets to reduce storage costs
5. **NAT Gateways**: Non-production environments use 0 NAT gateways by default

## Troubleshooting

### Stack Already Exists Error

If you get "Stack already exists" error:

1. Check if stack name conflicts with existing stack
2. Ensure `ENVIRONMENT` variable is set correctly
3. Verify environment configuration in `environments.json`

### IAM Role Permission Issues

If deployment fails with IAM errors:

1. Ensure Docker GitHub Action role has CDK bootstrap permissions
2. Verify the role can assume CDK execution roles
3. Check CloudFormation stack events for detailed error messages

### Frontend Deployment Issues

If frontend deployment fails:

1. Verify IAM role ARN is correct for the environment
2. Check S3 bucket exists and is in correct region
3. Ensure CloudFront distribution origin is properly configured
4. Check version.json file is uploaded to S3 bucket

### Resource Quota Limits

AWS has service quotas that may affect multi-environment deployments:

1. **VPCs**: Default limit is 5 per region
2. **Elastic IPs**: May be limited in your account
3. **ECR Repositories**: Ensure sufficient quota for all service images
4. **CloudFormation Stacks**: Default limit is 200 stacks per account

Request quota increases through AWS Support if needed.

## Monitoring and Observability

Each environment has independent monitoring:

- **CloudWatch Logs**: `/glowing-telegram/{environment}/lambda/*`
- **CloudWatch Metrics**: Namespaced by environment via tags
- **X-Ray Tracing**: Enabled for all Lambda functions
- **DynamoDB Streams**: Monitor for real-time data changes

## Security Considerations

1. **Secrets Isolation**: Each environment has separate Secrets Manager secrets
2. **IAM Roles**: Environment-specific roles prevent cross-environment access
3. **Network Isolation**: Each environment has its own VPC and security groups
4. **Database Isolation**: Separate RDS clusters and DynamoDB tables per environment
5. **Access Control**: Use IAM policies to restrict who can deploy to production

## Best Practices

1. **Test in dev first**: Always test changes in dev before promoting to staging/production
2. **Use staging for release testing**: Deploy release candidates to staging before production
3. **Immutable deployments**: Use tagged Docker images rather than rebuilding
4. **Infrastructure as Code**: All changes should go through CDK, avoid manual AWS Console changes
5. **Monitor deployments**: Check CloudWatch logs and metrics after each deployment
6. **Rollback strategy**: Keep previous Docker image tags available for quick rollback
7. **Database migrations**: Test data migrations in dev/staging before production
8. **Backup strategy**: Ensure proper backup configuration for production databases

## Future Enhancements

Potential improvements to the multi-environment system:

1. **Automated promotion**: GitHub Action to promote tested staging builds to production
2. **Blue-green deployments**: Zero-downtime deployments with traffic shifting
3. **Canary deployments**: Gradual rollout to subset of users
4. **Environment cloning**: Copy production data to staging for testing
5. **Cost dashboards**: Per-environment cost tracking and alerts
6. **Integration tests**: Automated smoke tests after each deployment
7. **Secrets rotation**: Automated rotation of API keys and credentials
8. **Multi-region**: Deploy environments across multiple AWS regions
