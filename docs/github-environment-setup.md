# GitHub Environment Setup for Frontend Repository

This guide explains how to configure GitHub Environments in the `glowing-telegram-frontend` repository for automated deployments.

## Prerequisites

1. Backend infrastructure must be deployed first (creates required AWS resources)
2. CDK stacks must be deployed to the target environment (dev, staging, or production)
3. You need admin access to the `glowing-telegram-frontend` repository

## Step 1: Deploy Backend Infrastructure

First, deploy the backend infrastructure for your target environment:

```bash
# For dev environment
cd glowing-telegram/cdk
ENVIRONMENT=dev IMAGE_VERSION=latest npm run cdk deploy --all

# For staging environment
ENVIRONMENT=staging IMAGE_VERSION=latest npm run cdk deploy --all

# Production is deployed via GitHub releases
```

## Step 2: Extract Stack Outputs

Use the `export-stack-outputs.sh` script to extract configuration values from deployed stacks:

```bash
cd glowing-telegram

# Export dev environment outputs
./scripts/export-stack-outputs.sh dev

# Export staging environment outputs
./scripts/export-stack-outputs.sh staging

# Export production environment outputs
./scripts/export-stack-outputs.sh production
```

The script will:
- Extract outputs from CloudFormation stacks
- Save JSON files to `frontend-config/` directory
- Display formatted configuration values for GitHub setup

Example output:
```
=== GitHub Environment Variables ===

Add these to GitHub Environment 'dev' in glowing-telegram-frontend:

Secrets:
  AWS_ROLE_ARN: arn:aws:iam::159222827421:role/GlowingTelegram-GithubActionRole-dev
  FRONTEND_BUCKET: glowing-telegram-frontend-dev

Variables:
  API_URL: https://abc123.execute-api.us-west-2.amazonaws.com
  WEBSOCKET_URL: wss://xyz789.execute-api.us-west-2.amazonaws.com
  USER_POOL_ID: us-west-2_XXXXXXXXX
  USER_POOL_CLIENT_ID: 1a2b3c4d5e6f7g8h9i0j
  CLOUDFRONT_DOMAIN: d1234567890abc.cloudfront.net
```

## Step 3: Create GitHub Environment

In the `glowing-telegram-frontend` repository:

1. Go to **Settings** → **Environments**
2. Click **New environment**
3. Enter environment name (must match backend: `dev`, `staging`, or `production`)
4. Click **Configure environment**

## Step 4: Configure Environment Secrets

Add the following secrets (use values from Step 2):

### AWS_ROLE_ARN
- **Description**: IAM Role ARN for GitHub Actions to assume when deploying frontend assets
- **Value**: Copy from `export-stack-outputs.sh` output (RepoStack output)
- **Example**: `arn:aws:iam::159222827421:role/GlowingTelegram-GithubActionRole-dev`

### FRONTEND_BUCKET
- **Description**: S3 bucket name where frontend assets will be uploaded
- **Value**: Copy from `export-stack-outputs.sh` output (FrontendStack output)
- **Example**: `glowing-telegram-frontend-dev`

## Step 5: Configure Environment Variables

Add the following variables (use values from Step 2):

### API_URL
- **Description**: HTTP API Gateway endpoint for backend CRUD operations
- **Value**: Copy from `export-stack-outputs.sh` output (AppStack output)
- **Example**: `https://abc123.execute-api.us-west-2.amazonaws.com`

### WEBSOCKET_URL
- **Description**: WebSocket API Gateway endpoint for real-time features
- **Value**: Copy from `export-stack-outputs.sh` output (AppStack output)
- **Example**: `wss://xyz789.execute-api.us-west-2.amazonaws.com`

### USER_POOL_ID
- **Description**: Cognito User Pool ID for authentication
- **Value**: Copy from `export-stack-outputs.sh` output (AppStack output)
- **Example**: `us-west-2_XXXXXXXXX`

### USER_POOL_CLIENT_ID
- **Description**: Cognito User Pool Client ID for authentication
- **Value**: Copy from `export-stack-outputs.sh` output (AppStack output)
- **Example**: `1a2b3c4d5e6f7g8h9i0j`

### CLOUDFRONT_DOMAIN
- **Description**: CloudFront distribution domain name for frontend assets
- **Value**: Copy from `export-stack-outputs.sh` output (FrontendStack output)
- **Example**: `d1234567890abc.cloudfront.net`

### AWS_REGION (Optional)
- **Description**: AWS region where resources are deployed
- **Value**: `us-west-2` (or your configured region)
- **Default**: Usually not needed as it's in the API URLs

## Step 6: Configure Environment Protection Rules (Optional)

For production and staging environments, you may want to add protection rules:

1. In the environment settings, scroll to **Environment protection rules**
2. Enable options as needed:
   - **Required reviewers**: Require manual approval before deployment
   - **Wait timer**: Add a delay before deployment
   - **Deployment branches**: Restrict which branches can deploy to this environment

Example protection rules:
- **Production**: 
  - Required reviewers: 1-2 team members
  - Deployment branches: Only `main` branch
- **Staging**: 
  - Required reviewers: Optional
  - Deployment branches: `main` and `develop`
- **Dev**: 
  - No protection rules (allow any branch)

## Step 7: Verify Configuration

Test the environment configuration:

1. Go to **Actions** tab in `glowing-telegram-frontend`
2. Find the deployment workflow
3. Click **Run workflow**
4. Select your newly configured environment
5. Watch the workflow run and verify:
   - AWS credentials authenticate successfully
   - Frontend assets upload to S3
   - Application builds correctly with environment variables

## Troubleshooting

### "Unable to assume role" error

**Problem**: GitHub Actions can't assume the AWS IAM role

**Solutions**:
1. Verify the `AWS_ROLE_ARN` secret is correct
2. Check that the IAM role exists in AWS Console
3. Verify the role's trust policy allows GitHub OIDC provider
4. Ensure environment name matches in GitHub and trust policy

### "Bucket does not exist" error

**Problem**: S3 bucket not found

**Solutions**:
1. Verify `FRONTEND_BUCKET` secret is correct
2. Check that FrontendStack deployed successfully
3. Run `aws s3 ls | grep glowing-telegram-frontend` to list buckets
4. Ensure you're in the correct AWS region

### API calls fail with CORS errors

**Problem**: Frontend can't communicate with backend

**Solutions**:
1. Verify `API_URL` and `WEBSOCKET_URL` variables are correct
2. Check that AppStack deployed successfully
3. Verify the URLs are accessible: `curl -I <API_URL>`
4. Check API Gateway CORS configuration in CDK

### Authentication fails

**Problem**: Users can't sign in

**Solutions**:
1. Verify `USER_POOL_ID` and `USER_POOL_CLIENT_ID` are correct
2. Check Cognito User Pool exists in AWS Console
3. Verify User Pool Client has correct OAuth settings
4. Check redirect URLs are configured in Cognito

### CloudFront shows 403/404 errors

**Problem**: Frontend assets not loading

**Solutions**:
1. Verify `CLOUDFRONT_DOMAIN` variable is correct
2. Check CloudFront distribution status (must be "Deployed")
3. Verify S3 bucket has frontend assets uploaded
4. Check `config/version.json` exists in S3 bucket
5. Wait for CloudFront cache to clear (5-15 minutes)

## Environment Synchronization

When backend infrastructure changes:

1. Re-deploy the backend stack to update resources
2. Re-run `export-stack-outputs.sh` to get updated values
3. Update GitHub Environment secrets/variables if values changed
4. Trigger a frontend deployment to pick up changes

## Multiple Environments Workflow

Typical workflow with multiple environments:

1. **Development**:
   - Developers push feature branches
   - Deploy to `dev` environment for testing
   - API changes can be tested immediately
   
2. **Staging**:
   - Merge to `develop` or `staging` branch
   - Deploy to `staging` environment
   - QA team tests before production release
   
3. **Production**:
   - Create GitHub release
   - Automatically deploys to `production`
   - Customers see changes

## Security Best Practices

1. **Secrets Management**:
   - Never commit secrets to repository
   - Use GitHub Environments for environment-specific secrets
   - Rotate AWS credentials periodically

2. **IAM Permissions**:
   - Use least-privilege IAM roles
   - Separate roles per environment
   - Audit role usage regularly

3. **Environment Protection**:
   - Require approvals for production deployments
   - Limit who can configure environments
   - Use branch protection rules

4. **Access Control**:
   - Limit who can deploy to production
   - Use GitHub teams for environment access
   - Audit deployment history regularly

## Automation Options

### Automatic Deployments

Configure workflows to automatically deploy when:
- Code is merged to `main` → deploy to production
- Code is merged to `develop` → deploy to staging
- Any branch push → deploy to dev

### Deployment Notifications

Add notifications for deployment events:
- Slack/Discord webhooks
- Email notifications
- GitHub deployment status

### Rollback Procedures

Implement rollback workflows:
- Keep previous versions tagged
- Deploy previous frontend version quickly
- Coordinate with backend version

## Related Documentation

- [Initial Setup Guide](INITIAL_SETUP.md) - First-time deployment
- [Normal Operation Guide](NORMAL_OPERATION.md) - Day-to-day workflows
- [Main README](../README.md) - Project overview
- [CDK Documentation](../cdk/README.md) - Infrastructure details

## Support

If you encounter issues:
1. Check CloudWatch logs for backend errors
2. Check GitHub Actions logs for deployment errors
3. Verify AWS resources exist and are in correct state
4. Review this documentation for configuration requirements
5. Ask in team chat or create GitHub issue
