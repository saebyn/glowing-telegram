# Dynamic Frontend Version Configuration with Lambda Updates

This document describes the dynamic frontend version configuration feature implemented using S3 event-triggered Lambda functions to update CloudFront distribution configurations.

## Overview

The dynamic frontend version configuration feature allows you to deploy specific frontend versions through CloudFront by automatically updating the origin path whenever the configuration file changes. This approach provides:

1. Real-time origin path updates triggered by S3 config changes
2. No CDK deployments required for version changes
3. Automatic CloudFront distribution updates via AWS SDK
4. Rapid version deployment (1-2 minutes vs 5-10 minutes for CDK)

## Architecture

```
S3 Config Update → S3 Event → Lambda Function → CloudFront API → Updated Origin Path
     ↓                           ↓                      ↓
config/version.json    Origin Updater Function    Distribution Config
```

### Components

1. **CloudFront Distribution**
   - Configured with origin path that gets updated dynamically
   - Initial path set from CDK deployment
   - Subsequent updates handled by Lambda function

2. **Origin Updater Lambda Function**
   - Triggered by S3 events when config/version.json changes
   - Reads new version from config file
   - Updates CloudFront distribution's origin path via AWS SDK
   - Creates cache invalidation for immediate updates

3. **Version Configuration File**: `config/version.json`
   - Stored in the frontend assets S3 bucket
   - Changes trigger Lambda function execution
   - Contains current version and metadata

4. **S3 Event Notifications**
   - Monitor changes to `config/version.json`
   - Trigger Lambda function on OBJECT_CREATED and OBJECT_REMOVED events

## Version Configuration File

The version configuration file is stored at `config/version.json` in your S3 bucket:

```json
{
  "version": "0.4.0",
  "description": "Current frontend version configuration",
  "lastUpdated": "2024-01-01T00:00:00Z",
  "rollbackVersion": "0.3.0",
  "metadata": {
    "deployedBy": "system",
    "environment": "production"
  }
}
```

### Required Fields

- `version`: The frontend version to serve (e.g., "1.2.3")

### Optional Fields

- `description`: Human-readable description
- `lastUpdated`: ISO timestamp of last update
- `rollbackVersion`: Previous version for rollback scenarios
- `metadata`: Additional deployment metadata

## Deployment

### Initial Setup

The dynamic version selection is automatically deployed with the frontend stack:

```bash
cd cdk
npm run build
cdk deploy FrontendStack
```

This deploys:
- CloudFront distribution with initial origin path
- Lambda function for updating CloudFront
- S3 event notifications for config changes
- IAM permissions for CloudFront updates

### Updating Version Configuration

To change the frontend version, simply update the config file:

1. **Using the update script**:
   ```bash
   ./scripts/update-frontend-version.sh 1.2.0
   ```
   This script will:
   - Update `config/version.json` in S3
   - Automatically trigger Lambda function via S3 event
   - CloudFront distribution will be updated within 1-2 minutes

2. **Manual process**:
   ```bash
   # Update version config - this triggers automatic CloudFront update
   echo '{"version": "1.2.0"}' | aws s3 cp - s3://your-bucket/config/version.json
   ```

3. **Wait for propagation**: 
   - Lambda function updates CloudFront: ~30-60 seconds
   - CloudFront edge cache propagation: 5-15 minutes (same as before)

### Rollback

To rollback to a previous version:

```bash
./scripts/rollback-frontend-version.sh
```

This reads the `rollbackVersion` from the current config and automatically updates the distribution.

## Performance Considerations

### Update Speed

1. **Config File Update**: Immediate (S3 PUT operation)
2. **Lambda Function Execution**: 30-60 seconds
3. **CloudFront Distribution Update**: 1-2 minutes
4. **Edge Cache Propagation**: 5-15 minutes (unchanged)

### Latency Impact

- No additional request latency (no runtime processing)
- Standard CloudFront performance characteristics
- Cache invalidation ensures immediate updates after distribution update

## Lambda Function Details

The Origin Updater Lambda function:

```python
def handler(event, context):
    """
    Lambda function to update CloudFront distribution origin path
    when config/version.json is updated in S3
    """
    # Process S3 event records
    # Read new version from config file
    # Update CloudFront distribution configuration
    # Create cache invalidation
```

### Function Permissions

The Lambda function has the following IAM permissions:
- `cloudfront:GetDistribution`
- `cloudfront:GetDistributionConfig` 
- `cloudfront:UpdateDistribution`
- `cloudfront:CreateInvalidation`
- `s3:GetObject` (for reading config file)

## Monitoring and Troubleshooting

### CloudWatch Logs

Lambda function logs are available in CloudWatch Logs under `/aws/lambda/[FunctionName]`.

Common log messages:
- `Retrieved version from S3: X.Y.Z` - Successfully read version
- `Updating CloudFront distribution to version: X.Y.Z` - Starting update
- `CloudFront distribution updated successfully` - Update completed
- `Cache invalidation created: [ID]` - Cache cleared

### Common Issues

1. **Version not updating**
   - Check Lambda function logs in CloudWatch
   - Verify S3 event was triggered
   - Ensure version.json is properly formatted JSON

2. **Lambda Function Errors**
   - Review CloudWatch Logs for error details
   - Check IAM permissions for CloudFront access
   - Verify S3 object accessibility

3. **CloudFront Update Failures**
   - Check if another update is in progress
   - Verify distribution exists and is accessible
   - Review CloudFront service limits

### Testing

Test the CDK stack configuration:

```bash
cd cdk
npm run build
npm test
```

Test the Lambda function integration:

```bash
# Update version and monitor logs
./scripts/update-frontend-version.sh 1.0.0

# Check CloudWatch logs
aws logs tail /aws/lambda/[FunctionName] --follow
```

## Security Considerations

### Lambda Function Security

The Lambda function uses:
- Least privilege IAM permissions (only CloudFront and S3 read access)
- No sensitive data in environment variables
- Secure S3 event integration

### S3 Bucket Access

The S3 bucket uses Origin Access Control (OAC) for secure access from CloudFront.

### CloudFront Updates

- Updates are performed using AWS SDK with proper authentication
- Only the origin path is modified, not security settings
- Function validates version format before applying changes

## Future Enhancements

This foundation supports future features:

1. **A/B Testing**
   ```json
   {
     "version": "1.0.0",
     "abTest": {
       "enabled": true,
       "variants": {
         "control": { "version": "1.0.0", "percentage": 50 },
         "experimental": { "version": "1.1.0", "percentage": 50 }
       }
     }
   }
   ```

2. **Feature Flags**
   ```json
   {
     "version": "1.0.0",
     "features": {
       "newDashboard": true,
       "betaFeature": false
     }
   }
   ```

3. **Staged Rollouts**
   ```json
   {
     "version": "1.0.0",
     "rollout": {
       "enabled": true,
       "percentage": 25,
       "targetVersion": "1.1.0"
     }
   }
   ```

## Cost Analysis

### Lambda Function Costs

- **Invocations**: Minimal cost (only triggered on config changes)
- **Duration**: ~3-5 seconds per execution
- **Memory**: 128MB (default)
- **Estimated cost**: <$1/month for typical usage

### Additional Costs

1. **CloudWatch Logs**: Minimal cost for function logs
2. **CloudFront API Calls**: No additional cost for distribution updates
3. **S3 Events**: No additional cost for notifications

### Cost Optimization

- No Lambda@Edge execution costs during request processing
- Faster updates reduce operational overhead
- Simplified architecture with fewer moving parts

## Migration from Static Version

To migrate from the previous static CDK approach:

1. Deploy the updated CDK stack (this change)
2. Verify S3 event notifications are working
3. Test version change with update script
4. Monitor Lambda function logs for successful execution

The migration adds the Lambda function and S3 event triggers while maintaining the existing S3 bucket and CloudFront distribution.