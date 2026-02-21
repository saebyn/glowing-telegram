# Multi-Environment Deployment - Normal Operation

This guide covers day-to-day deployment workflows after completing [Initial Setup](INITIAL_SETUP.md). All deployments happen through GitHub Actions - no manual CLI deployments are needed.

## Quick Reference

| Task                         | Workflow              | When to Use                           |
| ---------------------------- | --------------------- | ------------------------------------- |
| Deploy feature to dev        | Deploy to Environment | Testing feature branches before merge |
| Deploy release to production | Automatic on release  | Publishing new version to users       |
| Rollback production          | Deploy to Environment | Emergency rollback needed             |
| Update dev with main         | Deploy to Environment | Sync dev with latest main             |

## Deploying Feature Branches to Dev

Use this workflow to test feature branches in the dev environment before merging to main.

### Step-by-Step Process

1. **Push your feature branch** to GitHub
   ```bash
   git push origin feature/my-feature
   ```

2. **Go to GitHub Actions**
   - Navigate to `saebyn/glowing-telegram` repository
   - Click **Actions** tab
   - Select **"Deploy to Environment"** workflow

3. **Run the workflow**
   - Click **Run workflow** button
   - Configure:
     - **Branch**: `feature/my-feature` (your feature branch)
     - **Environment**: `dev`
     - **Image tag**: Leave empty (will build from branch)
   - Click **Run workflow**

4. **Monitor deployment**
   - Workflow takes approximately 15-20 minutes
   - Watch progress through the phases:
     - Phase 1: Deploy infrastructure dependencies (FrontendStack-dev, RepoStack-dev)
     - Phase 2: Build Docker images from your branch
     - Phase 3: Deploy application stack (AppStack-dev)
     - Phase 4: Configure GitHub environments

5. **Test in dev environment**
   - Access the dev frontend via CloudFront URL
   - Test your feature changes
   - Check CloudWatch logs for any errors

### What Happens During Deployment

- Docker images are built from your feature branch code
- Images are tagged as: `dev-feature-my-feature-YYYYMMDD-HHMMSS`
- Lambda functions are updated with new images
- Database migrations run automatically (if any)
- Frontend environment variables stay in sync

### After Testing

If your feature works correctly in dev:
1. Create a pull request to merge to `main`
2. Get code review and approval
3. Merge to `main`
4. Create a release to deploy to production

## Deploying to Production

Production deployments are fully automated when you create a GitHub release.

### Creating a Release

1. **Ensure main branch is stable**
   ```bash
   # Make sure you've tested in dev first
   git checkout main
   git pull origin main
   ```

2. **Create a release on GitHub**
   - Go to `saebyn/glowing-telegram` → **Releases**
   - Click **"Create a new release"**
   - Click **"Choose a tag"**
   - Create new tag: `v1.2.3` (use semantic versioning)
   - Set release title: "Release v1.2.3"
   - Add release notes describing changes
   - Click **"Publish release"**

3. **Automatic deployment triggers**
   - GitHub Actions workflow starts automatically
   - Builds Docker images tagged with `v1.2.3`
   - Deploys to production environment
   - No manual intervention required

4. **Monitor deployment**
   - Go to **Actions** tab
   - Watch "Deploy to Production" workflow
   - Deployment takes approximately 20-25 minutes

5. **Verify production deployment**
   - Check production frontend loads correctly
   - Monitor CloudWatch metrics for errors
   - Verify new features work as expected

### Production Deployment Details

- Uses existing stack names: `AppStack`, `FrontendStack`, `RepoStack`
- Zero downtime deployment (Lambda aliases update gradually)
- Automatic rollback on CloudFormation failures
- Database migrations run before Lambda updates

## Syncing Dev with Main Branch

Keep dev environment up-to-date with the latest main branch changes:

1. Go to **Actions** → **"Deploy to Environment"**
2. Click **Run workflow**
3. Configure:
   - **Branch**: `main`
   - **Environment**: `dev`
   - **Image tag**: Leave empty
4. Click **Run workflow**

This rebuilds dev environment with latest main branch code.

## Rollback Procedures

### Rolling Back Production (Emergency)

If a production release causes issues:

1. **Option A: Deploy previous release tag**
   - Go to **Actions** → **"Deploy to Environment"**
   - Click **Run workflow**
   - Configure:
     - **Branch**: `main`
     - **Environment**: `production`
     - **Image tag**: `v1.2.2` (previous stable version)
   - Click **Run workflow**

2. **Option B: Revert and create hotfix release**
   ```bash
   # Revert the problematic commit
   git revert <commit-hash>
   git push origin main
   
   # Create hotfix release
   # Go to GitHub and create release v1.2.4
   ```

### Rolling Back Dev

Simply deploy a different branch or previous version:

```
Actions → Deploy to Environment
Branch: main (or previous feature branch)
Environment: dev
Image tag: (empty or specific previous tag)
```

## Monitoring Deployments

### CloudFormation Console

Monitor stack updates in real-time:

```bash
# Watch stack status
aws cloudformation describe-stacks \
  --stack-name AppStack-dev \
  --query 'Stacks[0].StackStatus' \
  --region us-west-2

# View stack events
aws cloudformation describe-stack-events \
  --stack-name AppStack-dev \
  --region us-west-2 \
  --max-items 20
```

### GitHub Actions Logs

- Each workflow phase has detailed logs
- Click on job names to see step-by-step output
- Check for errors in red-highlighted steps
- Download logs for offline analysis

### CloudWatch Logs

Lambda function logs are automatically created:

```
Log Group Pattern: /aws/lambda/AppStack-dev-<FunctionName>-*
```

View recent errors:

```bash
aws logs filter-log-events \
  --log-group-name /aws/lambda/AppStack-dev-MyFunction \
  --filter-pattern "ERROR" \
  --region us-west-2 \
  --max-items 10
```

## Common Workflows

### Testing a PR in Dev

```
1. Push feature branch
2. Run "Deploy to Environment" workflow
   - Branch: feature/my-feature
   - Environment: dev
3. Test thoroughly in dev
4. Get PR approval
5. Merge to main
```

### Releasing to Production

```
1. Merge all PRs to main
2. Test main branch in dev (optional but recommended)
3. Create GitHub release with version tag
4. Automatic deployment to production
5. Monitor and verify
```

### Hotfix Process

```
1. Create hotfix branch from main
2. Make minimal fix
3. Deploy to dev for quick test
4. Merge hotfix to main
5. Create hotfix release (e.g., v1.2.4)
6. Automatic production deployment
```

### Updating Dependencies

```
1. Update Cargo.toml or package.json on feature branch
2. Deploy to dev to verify builds work
3. Test updated dependencies thoroughly
4. Merge to main
5. Release as normal
```

## Working with Docker Images

### Image Tagging Strategy

Images are automatically tagged based on context:

| Context            | Tag Format                 | Example                            |
| ------------------ | -------------------------- | ---------------------------------- |
| Production release | `v{version}`               | `v1.2.3`                           |
| Dev deployment     | `dev-{branch}-{timestamp}` | `dev-feature-auth-20260221-143022` |
| Manual override    | Custom tag                 | `v1.2.3` (for rollback)            |

### Viewing Available Images

```bash
# List images for a specific service
aws ecr list-images \
  --repository-name video_ingestor \
  --region us-west-2 \
  --query 'imageIds[*].imageTag' \
  --output table

# Filter dev images
aws ecr list-images \
  --repository-name video_ingestor \
  --region us-west-2 \
  --query "imageIds[?contains(imageTag, 'dev')].imageTag"
```

### Using Pre-built Images

To deploy without rebuilding (useful for quick tests):

1. Find existing image tag from ECR or previous workflow
2. Run "Deploy to Environment" workflow
3. Set **Image tag**: `dev-feature-xyz-20260220-120000`
4. Deploy completes in ~10 minutes (skips build phase)

## Troubleshooting

### Deployment Stuck or Taking Too Long

**Normal timing**:
- Full deployment with build: 15-20 minutes
- Deployment with existing images: 8-10 minutes
- Production release: 20-25 minutes

**If stuck longer**:
1. Check CloudFormation console for stack status
2. Look for resources in CREATE_IN_PROGRESS state
3. Common causes:
   - VPC ENI attachment (can take 10+ minutes)
   - RDS cluster scaling (5-10 minutes)
   - Lambda functions waiting for container images

**Solution**: Usually just wait. If stuck over 45 minutes, cancel and retry.

### Workflow Fails in Phase 2 (Docker Build)

**Common causes**:
- Out of disk space on GitHub runner
- Network timeout pulling base images
- Build error in Rust code

**Solution**:
1. Check workflow logs for specific error
2. Test build locally:
   ```bash
   docker buildx bake -f docker-bake.hcl <service-name>
   ```
3. Fix issue and push new commit
4. Re-run workflow

### Workflow Fails in Phase 3 (CDK Deploy)

**Common causes**:
- CloudFormation resource limit hit
- IAM permission denied
- Resource dependency conflict

**Solution**:
1. Check CloudFormation events for error message
2. Go to AWS Console → CloudFormation → Select stack
3. View "Events" tab for detailed error
4. Fix underlying issue (may require AWS Console changes)
5. Re-run workflow

### Frontend Shows Old Version After Deployment

**Cause**: Browser cache or CloudFront cache

**Solution**:
1. Hard refresh browser (Ctrl+Shift+R or Cmd+Shift+R)
2. Check CloudFront invalidation was created
3. Verify version.json file was updated in S3:
   ```bash
   aws s3 cp s3://glowing-telegram-frontend-dev/version.json - --region us-west-2
   ```

### Lambda Functions Return 500 Errors

**Common causes**:
- Missing environment variables
- Secrets not populated
- Database connection error
- Docker image failed to load

**Solution**:
1. Check CloudWatch Logs for the specific Lambda
2. Common fixes:
   - Populate missing secrets in Secrets Manager
   - Verify VPC/security group configuration
   - Check Lambda has correct IAM permissions
   - Verify Docker image exists in ECR

### GitHub Environment Variables Not Updated

**Cause**: CDK couldn't access GitHub API

**Solution**:
1. Verify GitHub token in Secrets Manager is valid
2. Check CDK logs for GitHub API errors
3. Manually update variables if needed:
   - Go to repo Settings → Environments
   - Select environment
   - Edit variables

## Best Practices

### Before Merging to Main

- [ ] Deploy feature branch to dev
- [ ] Test thoroughly in dev environment
- [ ] Check CloudWatch logs for errors
- [ ] Get code review approval
- [ ] Ensure CI tests pass

### Before Creating Release

- [ ] Verify main branch is stable
- [ ] Consider deploying main to dev first
- [ ] Prepare release notes
- [ ] Check for breaking changes
- [ ] Plan rollback strategy if needed

### After Production Deploy

- [ ] Monitor CloudWatch metrics for 10-15 minutes
- [ ] Check error rates in CloudWatch Logs
- [ ] Test critical user flows
- [ ] Verify database migrations completed
- [ ] Update team on deployment status

### Regular Maintenance

- **Weekly**: Review CloudWatch logs for errors
- **Monthly**: Check ECR for old unused images (cleanup)
- **Quarterly**: Review AWS costs per environment
- **As needed**: Rotate secrets (GitHub tokens, API keys)

## Cost Optimization

### Dev Environment Costs

To minimize dev environment costs:

- Deploy only when actively testing
- Use smaller Lambda memory settings (configured in CDK)
- Aurora Serverless automatically scales down when idle
- Frontend assets in S3 are low cost

### Cleanup Old Images

Remove old dev Docker images periodically:

```bash
# List images older than 30 days
aws ecr list-images \
  --repository-name video_ingestor \
  --region us-west-2 \
  --query "imageIds[?contains(imageTag, 'dev')]" \
  --output json

# Delete specific image
aws ecr batch-delete-image \
  --repository-name video_ingestor \
  --image-ids imageTag=dev-old-branch-20260101-120000 \
  --region us-west-2
```

## Environment Differences

### Production vs Dev

| Aspect       | Production                | Dev                  |
| ------------ | ------------------------- | -------------------- |
| Stack Names  | AppStack                  | AppStack-dev         |
| Frontend URL | production-domain.com     | dev CloudFront URL   |
| Database     | Aurora production cluster | Aurora dev cluster   |
| Logging      | 90 day retention          | 7 day retention      |
| Monitoring   | Full alerting enabled     | Basic logging only   |
| Secrets      | Production credentials    | Test/dev credentials |

### Configuration Drift

Keep configurations in sync:
- API integrations (Twitch, YouTube) use separate dev apps
- OAuth redirect URLs point to correct environment
- Webhook endpoints configured per environment
- Test data in dev, real data in production

## Getting Help

### Debugging Checklist

1. **Check GitHub Actions logs** - See which phase failed
2. **Check CloudFormation events** - See AWS resource errors  
3. **Check CloudWatch Logs** - See Lambda runtime errors
4. **Check ECR repositories** - Verify images exist
5. **Check Secrets Manager** - Verify secrets populated

### Escalation Path

If deployment fails repeatedly:
1. Review error messages in all logs
2. Check AWS Service Health Dashboard
3. Verify IAM permissions haven't changed
4. Consider manual CloudFormation rollback
5. File GitHub issue with full error context

## Reference

### Environment Variables Set Automatically

GitHub environments are automatically configured with:

**Backend repo** (`glowing-telegram`):
- `AWS_REGION`: us-west-2
- `ECR_REGISTRY`: Account ECR URL

**Frontend repo** (`glowing-telegram-frontend`):
- `API_URL`: API Gateway endpoint
- `WEBSOCKET_URL`: WebSocket API endpoint
- `COGNITO_USER_POOL_ID`: User pool for auth
- `COGNITO_USER_POOL_CLIENT_ID`: App client ID
- `BUCKET_NAME`: Frontend asset bucket
- And more...

### Useful AWS CLI Commands

```bash
# Get stack outputs
aws cloudformation describe-stacks \
  --stack-name AppStack-dev \
  --query 'Stacks[0].Outputs'

# List all stacks
aws cloudformation list-stacks \
  --stack-status-filter CREATE_COMPLETE UPDATE_COMPLETE

# Get Lambda function info
aws lambda get-function \
  --function-name AppStack-dev-VideoIngestor

# Check RDS cluster status
aws rds describe-db-clusters \
  --query "DBClusters[?contains(DBClusterIdentifier, 'dev')]"
```

---

**Last Updated**: February 2026  
**Version**: 1.0.0 (Initial multi-environment support)
