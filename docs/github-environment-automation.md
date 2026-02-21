# GitHub Environment Automation

This document explains how the automated GitHub environment management works in the glowing-telegram project.

## Overview

The CDK now automatically creates and updates GitHub environments in both the `glowing-telegram` (backend) and `glowing-telegram-frontend` repositories. This eliminates manual configuration and ensures environments are always in sync with deployed infrastructure.

## How It Works

### Architecture

1. **GitHubEnvironmentManager**: A CDK custom resource construct that uses a Lambda function to interact with the GitHub API
2. **GitHubEnvironmentStack**: A CDK stack that orchestrates environment creation for both repositories
3. **Lambda Function**: Executes on stack deployment to create/update GitHub environments and their variables via GitHub REST API

### Workflow

```
CDK Deployment
    ↓
GitHubEnvironmentStack
    ↓
GitHubEnvironmentManager (Custom Resource)
    ↓
Lambda Function
    ↓
GitHub API
    ↓
GitHub Environments Created/Updated
```

## Prerequisites

### 1. GitHub Personal Access Token

Create a GitHub Personal Access Token (classic) with the following scopes:
- `repo` (Full control of private repositories)
- `workflow` (Update GitHub Action workflows)

### 2. Store Token in Secrets Manager

Store the token in AWS Secrets Manager:

```bash
aws secretsmanager create-secret \
  --name glowing-telegram/github-token \
  --description "GitHub PAT for environment automation" \
  --secret-string '{"token":"ghp_your_token_here"}'
```

Or via AWS Console:
1. Go to AWS Secrets Manager
2. Create new secret
3. Choose "Other type of secret"
4. Add key: `token`, value: your GitHub PAT
5. Name: `glowing-telegram/github-token`

## Variables Set Automatically

### Frontend Repository (glowing-telegram-frontend)

The following variables are set for each environment:

| Variable             | Description                                                  | Source                         |
| -------------------- | ------------------------------------------------------------ | ------------------------------ |
| API_URL              | HTTP API Gateway endpoint                                    | AppStack output                |
| AWS_REGION           | AWS region for deployment                                    | Environment config             |
| AWS_ROLE_ARN         | IAM role for GitHub Actions                                  | RepoStack output               |
| BUCKET_NAME          | S3 bucket for frontend assets                                | FrontendStack output           |
| COGNITO_CLIENT_ID    | Cognito User Pool Client ID                                  | AppStack output                |
| COGNITO_DOMAIN       | Cognito User Pool domain                                     | AppStack output                |
| COGNITO_USER_POOL_ID | Cognito User Pool ID                                         | AppStack output                |
| CONTENT_URL          | CloudFront distribution URL                                  | AppStack output                |
| LOGOUT_URI           | OAuth logout redirect URI                                    | AppStack output                |
| REDIRECT_URI         | OAuth callback redirect URI                                  | AppStack output                |
| SITE_DOMAIN          | CloudFront domain name (e.g., d3qqtvukmpw4yh.cloudfront.net) | FrontendStack output           |
| TWITCH_CLIENT_ID     | Twitch application client ID                                 | Environment variable or secret |
| WEBSOCKET_URL        | WebSocket API Gateway endpoint                               | AppStack output                |

### Backend Repository (glowing-telegram)

The following variables are set for each environment:

| Variable    | Description                                 |
| ----------- | ------------------------------------------- |
| ENVIRONMENT | Environment name (dev, staging, production) |
| AWS_REGION  | AWS region for deployment                   |

## Deployment

### Automatic Environment Creation

When you deploy the CDK stacks, environments are automatically created/updated:

```bash
cd cdk

# Deploy all stacks including GitHub environments
ENVIRONMENT=dev IMAGE_VERSION=latest npm run cdk deploy --all

# The GitHubEnvironmentStack will:
# 1. Create/update 'dev' environment in glowing-telegram repo
# 2. Create/update 'dev' environment in glowing-telegram-frontend repo
# 3. Set all required variables in both environments
```

### Skip Environment Automation

If you want to skip GitHub environment creation (e.g., for testing):

```bash
SKIP_GITHUB_ENV=true ENVIRONMENT=dev npm run cdk deploy --all
```

### Deployment Order

The CDK ensures proper dependency order:

1. FrontendStack (creates S3 bucket, CloudFront)
2. RepoStack (creates IAM roles)
3. AppStack (creates APIs, Cognito, etc.)
4. GitHubEnvironmentStack (creates GitHub environments with all values)

## GitHub Environment Integration

### Using Environments in Workflows

The deploy-environment.yml workflow now uses GitHub environments:

```yaml
on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Target environment to deploy to'
        required: true
        type: environment  # Uses GitHub environments
      # ... other inputs

jobs:
  build-images:
    runs-on: ubuntu-latest
    environment: ${{ inputs.environment }}  # Declares environment usage
    # ...
  
  deploy:
    runs-on: ubuntu-latest
    environment: ${{ inputs.environment }}  # Declares environment usage
    # ...
```

### Benefits of GitHub Environments

1. **Protection Rules**: Require approvals before deploying to production
2. **Branch Restrictions**: Limit which branches can deploy to each environment
3. **Secrets**: Store environment-specific secrets (handled separately from variables)
4. **Audit Trail**: Track who deployed what to which environment
5. **Visual Indicators**: See active deployments in GitHub UI

### Configuring Environment Protection

After CDK creates the environments, you can add protection rules via GitHub UI:

1. Go to repository → Settings → Environments
2. Click on environment name (e.g., "production")
3. Configure:
   - **Required reviewers**: Add team members who must approve
   - **Wait timer**: Add delay before deployment
   - **Deployment branches**: Restrict to specific branches
   - **Environment secrets**: Add secrets (not variables)

Example for production:
- Required reviewers: 1-2 maintainers
- Deployment branches: Only `main`
- Wait timer: 0 (or add delay if desired)

## Troubleshooting

### Issue: Lambda function fails with authentication error

**Cause**: GitHub token not found or invalid

**Solution**:
1. Verify secret exists: `aws secretsmanager describe-secret --secret-id glowing-telegram/github-token`
2. Verify token has correct scopes (repo, workflow)
3. Token must not be expired
4. Check Lambda logs in CloudWatch: `/glowing-telegram/custom-resources/github-env-manager-*`

### Issue: Variables not updated in GitHub

**Cause**: Custom resource didn't trigger update

**Solution**:
1. Check CloudWatch logs for the Lambda function
2. Verify the custom resource received the event
3. Try forcing an update by changing a stack value
4. Manually trigger update by modifying GitHubEnvironmentStack

### Issue: GitHub API rate limiting

**Cause**: Too many API calls

**Solution**:
1. GitHub API has rate limits (5000 requests/hour for authenticated requests)
2. Each deployment makes ~15-20 API calls
3. Should not be an issue for normal use
4. If hit limit, wait for rate limit reset (check response headers)

### Issue: Environment not visible in GitHub

**Cause**: Environment may not show until first deployment

**Solution**:
1. Run a workflow that uses the environment
2. Or deploy using the environment in a GitHub Action
3. Environment will appear after first use

### Issue: TWITCH_CLIENT_ID shows placeholder value

**Cause**: Twitch client ID not configured

**Solution**:

Option 1 - Use environment variable:
```bash
TWITCH_CLIENT_ID=your_client_id npm run cdk deploy GitHubEnvironmentStack-dev
```

Option 2 - Store in Secrets Manager:
```bash
aws secretsmanager create-secret \
  --name glowing-telegram/twitch-client-id \
  --secret-string "your_twitch_client_id"
```

Then redeploy the GitHubEnvironmentStack.

## Maintenance

### Updating Variables

Variables are updated automatically on each CDK deployment. No manual action needed.

### Adding New Variables

To add a new variable:

1. Update `GitHubEnvironmentStack` to include the new variable:
```typescript
const frontendVariables: Record<string, string> = {
  // ... existing variables
  NEW_VARIABLE: 'value',
};
```

2. Deploy the stack:
```bash
npm run cdk deploy GitHubEnvironmentStack-dev
```

### Removing Environments

Environments are **not** automatically deleted when you destroy CDK stacks (to prevent accidental data loss).

To manually remove:

1. Via GitHub UI:
   - Go to Settings → Environments
   - Click environment → Delete environment

2. Via GitHub API:
```bash
curl -X DELETE \
  -H "Authorization: token YOUR_GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/saebyn/glowing-telegram/environments/dev
```

### Monitoring

Check CloudWatch Logs:
```bash
aws logs tail /glowing-telegram/custom-resources/github-env-manager-glowing-telegram-dev --follow
aws logs tail /glowing-telegram/custom-resources/github-env-manager-glowing-telegram-frontend-dev --follow
```

## Security Considerations

1. **GitHub Token Security**:
   - Token stored in Secrets Manager (encrypted at rest)
   - Lambda function has IAM permission to read secret
   - Token never logged or exposed in CloudFormation outputs
   - Rotate token periodically

2. **Least Privilege**:
   - Token needs only `repo` scope (not `admin:org`)
   - Lambda runs with minimal IAM permissions
   - Custom resource only has access to GitHub API

3. **Environment Variables**:
   - Variables are public within the repository (not secrets)
   - Sensitive values should use GitHub Secrets instead
   - Variables visible in workflow logs

4. **Rate Limiting**:
   - Authenticated requests: 5000/hour
   - Use single token across deployments
   - Custom resource batches API calls efficiently

## API Reference

### GitHubEnvironmentManager

Custom resource construct for managing GitHub environments.

**Props**:
```typescript
{
  owner: string;              // GitHub org/user
  repo: string;               // Repository name
  environmentName: string;    // Environment name
  variables: Record<string, string>;  // Variables to set
  githubTokenSecretArn: string;       // Secret ARN for GitHub token
}
```

**Example**:
```typescript
new GitHubEnvironmentManager(this, 'MyEnvironment', {
  owner: 'saebyn',
  repo: 'my-repo',
  environmentName: 'production',
  variables: {
    API_URL: 'https://api.example.com',
    REGION: 'us-west-2',
  },
  githubTokenSecretArn: 'arn:aws:secretsmanager:...',
});
```

### GitHubEnvironmentStack

Stack to orchestrate environment management for multiple repositories.

**Props**:
```typescript
{
  environmentName: string;
  apiUrl: string;
  websocketUrl: string;
  userPoolId: string;
  userPoolClientId: string;
  cognitoDomain: string;
  awsRegion: string;
  contentUrl: string;
  redirectUri: string;
  logoutUri: string;
  frontendBucketName: string;
  githubRoleArn: string;
  twitchClientId?: string;
}
```

## Related Documentation

- [Initial Setup](INITIAL_SETUP.md) - First-time deployment guide
- [Normal Operation](NORMAL_OPERATION.md) - Day-to-day workflows
- [GitHub Environment Setup](github-environment-setup.md) - Manual setup (now automated!)
- [CDK Documentation](../cdk/README.md) - Infrastructure details

## Future Enhancements

Potential improvements:

1. **Secret Management**: Automate setting GitHub Secrets (requires different API endpoint)
2. **Protection Rules**: Configure protection rules via CDK
3. **Branch Policies**: Set deployment branch restrictions automatically
4. **Multiple Tokens**: Support different tokens per repository
5. **Webhook Integration**: Notify on environment changes
6. **Rollback Support**: Track variable history for rollbacks
