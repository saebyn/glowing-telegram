# Multi-Environment Deployment - Initial Setup

This guide covers the one-time setup required to deploy the multi-environment infrastructure for the first time. After completing these steps once, you'll use the [Normal Operation](NORMAL_OPERATION.md) guide for day-to-day deployments.

## Overview

The glowing-telegram system now supports two isolated deployment environments:

- **production**: Live production environment (existing infrastructure)
- **dev**: Development environment for testing feature branches

Both environments are automatically deployed via GitHub Actions. Manual CLI deployments are not supported.

## Prerequisites

Before starting, ensure you have:

1. **AWS Access**: AWS account credentials with admin permissions
2. **GitHub Access**: Admin access to both repositories:
   - `saebyn/glowing-telegram` (backend)
   - `saebyn/glowing-telegram-frontend` (frontend)
3. **GitHub Personal Access Token**: Token with `repo` scope for automated environment setup

## Step 1: Create GitHub Personal Access Token

The CDK stacks automatically configure GitHub environments. To enable this, create a Personal Access Token:

1. Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Click "Generate new token (classic)"
3. Set expiration to 1 year
4. Select scopes: **`repo`** (Full control of private repositories)
5. Click "Generate token" and **copy the token immediately**

## Step 2: Store GitHub Token in AWS Secrets Manager

The GitHub token must be stored in AWS Secrets Manager before deploying:

```bash
aws secretsmanager create-secret \
  --name glowing-telegram/github-token \
  --description "GitHub token for automated environment configuration" \
  --secret-string '{"token":"ghp_YOUR_TOKEN_HERE"}' \
  --region us-west-2
```

**Important**: Replace `ghp_YOUR_TOKEN_HERE` with your actual token.

To verify the secret was created:

```bash
aws secretsmanager describe-secret \
  --secret-id glowing-telegram/github-token \
  --region us-west-2
```

## Step 3: Configure Environment Settings

Before deploying, you need to configure environment-specific settings in `cdk/config/environments.json`.

### Twitch Client ID

**Get a Twitch Client ID**:
1. Go to https://dev.twitch.tv/console
2. Register your application (create separate dev and production apps)
3. Copy the Client ID
4. Configure redirect URLs for your environment

### GitHub Owner

The GitHub owner/organization name where your repositories are hosted. Default is `"saebyn"`.

If you've forked this repository or are deploying under a different GitHub organization, update this value.

### Update Configuration

Edit `cdk/config/environments.json`:

```json
{
  "environments": {
    "production": {
      "awsAccount": "159222827421",
      "awsRegion": "us-west-2",
      "frontendVersion": "0.4.1",
      "twitchClientId": "your_production_twitch_client_id",
      "githubOwner": "saebyn",
      "tags": { ... }
    },
    "dev": {
      "awsAccount": "159222827421",
      "awsRegion": "us-west-2",
      "frontendVersion": "latest",
      "twitchClientId": "your_dev_twitch_client_id",
      "githubOwner": "saebyn",
      "tags": { ... }
    }
  }
}
```

**Important**: 
- We may or may not use different Twitch applications
  for dev and production environments
- Ensure `githubOwner` matches your GitHub username or organization name

## Step 4: Deploy Dev Environment

Deploy the dev environment infrastructure using GitHub Actions:

1. Go to `saebyn/glowing-telegram` → **Actions** → "Deploy to Environment"
2. Click **Run workflow**
3. Configure:
   - **Branch**: `copilot/deploy-to-multiple-environments` (or your feature branch)
   - **Environment**: `dev`
   - **Image tag**: Leave empty (will build from branch)
4. Click **Run workflow**

The workflow will take approximately 15-20 minutes and performs these steps:

1. **Phase 1**: Deploy FrontendStack-dev and RepoStack-dev
2. **Phase 2**: Build Docker images from the branch
3. **Phase 3**: Deploy AppStack-dev with all Lambda functions and services
4. **Phase 4**: Configure GitHub environments in both repositories automatically

### What Gets Created

The dev environment creates:

- **CloudFormation Stacks**: `FrontendStack-dev`, `RepoStack-dev`, `AppStack-dev`
- **S3 Buckets**: Separate buckets for frontend assets, input/output files
- **ECR Repositories**: Docker image storage (shared across environments)
- **DynamoDB Tables**: Isolated data storage for dev
- **Lambda Functions**: All services with `-dev` suffix in names
- **Aurora PostgreSQL**: Dedicated database cluster for dev
- **IAM Roles**: Environment-specific roles for GitHub Actions
- **GitHub Environments**: Automatically configured in both repos

## Step 5: Verify Dev Deployment

After deployment completes, verify the infrastructure:

### Check CloudFormation Stacks

```bash
# Verify all stacks deployed successfully
aws cloudformation describe-stacks \
  --stack-name AppStack-dev \
  --region us-west-2 \
  --query 'Stacks[0].StackStatus'

# Expected output: "CREATE_COMPLETE" or "UPDATE_COMPLETE"
```

### Check GitHub Environment Configuration

1. Go to `saebyn/glowing-telegram` → **Settings** → **Environments**
2. Verify `dev` environment exists with variables:
   - `AWS_REGION`
   - `ECR_REGISTRY`
3. Go to `saebyn/glowing-telegram-frontend` → **Settings** → **Environments**
4. Verify `dev` environment exists with variables:
   - `API_URL`
   - `WEBSOCKET_URL`
   - `COGNITO_USER_POOL_ID`
   - And others...

### Test Dev Frontend Deployment

1. Go to `saebyn/glowing-telegram-frontend` → **Actions**
2. Find the frontend deployment workflow
3. Run workflow with environment: `dev`
4. Verify frontend deploys successfully

## Step 6: Configure AWS Secrets

Some secrets must be manually populated after initial deployment:

### OpenAI API Key

```bash
# Find the secret ARN
aws secretsmanager list-secrets \
  --query "SecretList[?contains(Name, 'AppStack-dev-OpenAISecret')].ARN" \
  --output text \
  --region us-west-2

# Update the secret value
aws secretsmanager put-secret-value \
  --secret-id <ARN_FROM_ABOVE> \
  --secret-string "sk-your-openai-api-key" \
  --region us-west-2
```

### YouTube OAuth Credentials

```bash
# Format: JSON with client_id, client_secret, redirect_uris
aws secretsmanager put-secret-value \
  --secret-id AppStack-dev-YoutubeAppSecret-XXXXX \
  --secret-string '{
    "client_id": "your-client-id",
    "client_secret": "your-client-secret",
    "redirect_uris": ["https://your-dev-domain/callback"]
  }' \
  --region us-west-2
```

### Twitch EventSub Configuration

Backend Twitch integration (EventSub webhooks) uses separate credentials configured as Lambda environment variables. These are configured separately from the frontend Twitch Client ID.

## Step 7: Create Initial Cognito Users

Create admin users in the dev Cognito User Pool:

1. Go to AWS Console → Cognito → User Pools
2. Find user pool named `AppStack-dev-*`
3. Create users → Add users
4. Create your admin account with temporary password

## Step 8: Test Dev Environment

Perform basic smoke tests:

1. **Access Frontend**: Navigate to CloudFront distribution URL
2. **Login**: Test authentication with Cognito user
3. **API Test**: Verify CRUD operations work
4. **WebSocket Test**: Check real-time updates
5. **Lambda Test**: Invoke a test Lambda function

## Step 9: Deploy to Production

After validating the dev environment works, prepare for production deployment:

### Important Considerations

- **Production uses existing stack names** without suffixes (`AppStack`, not `AppStack-production`)
- **Backward compatible** - existing production infrastructure unchanged
- **Zero downtime** - deployment updates existing resources

### Production Deployment Process

Production deployments happen automatically when you create a GitHub release:

1. Merge your feature branch to `main`
2. Go to `saebyn/glowing-telegram` → **Releases** → **Create a new release**
3. Create tag: `v1.x.x` (following semantic versioning)
4. Publish release
5. GitHub Actions automatically:
   - Builds Docker images with release tag
   - Deploys to production environment
   - Configures production GitHub environment

**Note**: The first production deployment with multi-environment support will update stack resources but won't change behavior since production uses the same stack names.

## Troubleshooting

### Deployment Fails: Stack Already Exists

**Problem**: CloudFormation error about stack already existing.

**Solution**: This is expected for production. The workflow will update the existing stacks. For dev, ensure you're using the correct environment name.

```bash
# Check existing stacks
aws cloudformation list-stacks \
  --stack-status-filter CREATE_COMPLETE UPDATE_COMPLETE \
  --query "StackSummaries[?contains(StackName, 'dev')].StackName" \
  --region us-west-2
```

### GitHub Environment Not Created

**Problem**: GitHub environment doesn't exist after deployment.

**Solution**: Check CDK logs for errors during GitHub API calls.

```bash
# Verify GitHub token is accessible
aws secretsmanager get-secret-value \
  --secret-id glowing-telegram/github-token \
  --region us-west-2
```

**Common causes**:
- GitHub token expired or invalid
- Token lacks `repo` scope
- Secret not in correct format (must be JSON with "token" key)

### Docker Image Build Fails

**Problem**: Phase 2 of workflow fails to build images.

**Solution**: Check that ECR repositories exist (created in Phase 1).

```bash
# List ECR repositories
aws ecr describe-repositories \
  --region us-west-2 \
  --query 'repositories[*].repositoryName'
```

If repositories are missing, Phase 1 didn't complete successfully. Re-run the workflow.

### Lambda Functions Can't Access Secrets

**Problem**: Lambda functions throw errors about missing secrets.

**Solution**: Verify secrets are populated (not just created empty).

```bash
# Check secret value exists
aws secretsmanager get-secret-value \
  --secret-id <secret-name> \
  --region us-west-2 \
  --query 'SecretString'
```

### Frontend Can't Connect to API

**Problem**: Frontend shows connection errors or 401 unauthorized.

**Solution**: Verify GitHub environment variables are set correctly.

1. Check `API_URL` in `glowing-telegram-frontend` dev environment
2. Ensure CORS is configured in API Gateway
3. Verify Cognito User Pool ID and Client ID are correct

Get the correct values:

```bash
aws cloudformation describe-stacks \
  --stack-name AppStack-dev \
  --query 'Stacks[0].Outputs' \
  --region us-west-2
```

### Database Connection Errors

**Problem**: Lambdas can't connect to Aurora PostgreSQL.

**Solution**: Check VPC configuration and security groups.

```bash
# Verify RDS cluster is available
aws rds describe-db-clusters \
  --query "DBClusters[?contains(DBClusterIdentifier, 'appstack-dev')].Status" \
  --region us-west-2
```

Ensure:
- RDS cluster is in "available" state
- Lambda functions are in correct VPC subnets
- Security groups allow Lambda → RDS traffic

### CDK Synth Fails in CI

**Problem**: GitHub Actions CI fails during CDK synth.

**Solution**: This is expected during initial PR. The `unit-testing` environment allows synth without AWS account.

```bash
# Test locally
cd cdk
ENVIRONMENT=unit-testing npm run cdk synth
```

### Timeout During Deployment

**Problem**: Workflow times out after 30-60 minutes.

**Solution**: 
- Check CloudFormation console for stuck resources
- Most common: Lambda functions waiting for VPC ENIs
- Solution: Cancel workflow and re-run (ENIs will be ready second time)

## Post-Setup Validation Checklist

After completing setup, verify:

- [ ] Dev environment deployed successfully (all 3 stacks)
- [ ] GitHub environments configured in both repositories
- [ ] All secrets populated in Secrets Manager
- [ ] Cognito users created
- [ ] Frontend deploys and loads correctly
- [ ] API endpoints respond correctly
- [ ] WebSocket connection works
- [ ] Docker images tagged correctly in ECR
- [ ] Production deployment plan understood

## Next Steps

After successful initial setup:

1. Read [Normal Operation](NORMAL_OPERATION.md) guide for daily workflows
2. Document any environment-specific configuration
3. Set up monitoring dashboards (optional)
4. Configure backup schedules (optional)
5. Plan deployment schedule for feature branches

## Need Help?

- **CDK Errors**: Check CloudFormation console events for detailed messages
- **GitHub Actions**: Check workflow logs for each phase
- **Lambda Errors**: Check CloudWatch Logs in AWS Console
- **API Issues**: Test with curl or Postman to isolate frontend vs backend

## Architecture Reference

### Stack Dependencies

```
FrontendStack-dev
    └── Creates S3 bucket
        └── Used by RepoStack-dev

RepoStack-dev
    └── Creates ECR repositories
        └── Used by Docker image builds
            └── Used by AppStack-dev

AppStack-dev
    └── All application infrastructure
```

### Environment Naming

| Resource Type    | Production                | Dev                            |
| ---------------- | ------------------------- | ------------------------------ |
| Stack Names      | AppStack                  | AppStack-dev                   |
| S3 Buckets       | glowing-telegram-frontend | glowing-telegram-frontend-dev  |
| Lambda Functions | MyFunction                | MyFunction (with dev env vars) |
| IAM Roles        | GlowingTelegram-Role      | GlowingTelegram-Role-dev       |

### GitHub Environments

Each environment has separate secrets/variables:

- **Backend repo** (`glowing-telegram`): AWS credentials for deployment
- **Frontend repo** (`glowing-telegram-frontend`): API URLs, Cognito config, S3 buckets

---

**Last Updated**: February 2026  
**Version**: 1.0.0 (Initial multi-environment support)
