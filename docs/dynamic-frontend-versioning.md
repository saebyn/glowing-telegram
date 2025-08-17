# Static Frontend Version Configuration

This document describes the static frontend version configuration feature implemented using CloudFront origin paths.

## Overview

The static frontend version configuration feature allows you to deploy specific frontend versions through CloudFront by setting the origin path to point directly to the version folder. This approach provides:

1. Direct origin path configuration based on version configuration in S3
2. Simplified request processing without dynamic rewriting
3. Version management through CDK deployments
4. Elimination of Lambda@Edge execution overhead

## Architecture

```
CloudFront Request → CloudFront Distribution → S3 Origin (with version path)
                           ↑
                    Version Config (deployment time)
                    config/version.json
```

### Components

1. **CloudFront Distribution**
   - Configured with origin path pointing directly to version folder
   - No dynamic request processing required
   - Standard CDN caching benefits

2. **Version Configuration File**: `config/version.json`
   - Stored in the frontend assets S3 bucket
   - Read during CDK deployment to set origin path
   - Contains current version and metadata

3. **CDK Deployment Process**
   - Reads version configuration at deployment time
   - Sets CloudFront origin path to `/version-folder`
   - Deploys updated distribution configuration

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

### Updating Version Configuration

To change the frontend version:

1. **Update the version file**:
   ```bash
   ./scripts/update-frontend-version.sh 1.2.0
   ```
   This script will:
   - Update `config/version.json` in S3
   - Trigger CDK deployment to update CloudFront origin path
   - Verify the deployment was successful

2. **Manual process** (if automated script fails):
   ```bash
   # Update version config
   echo '{"version": "1.2.0"}' | aws s3 cp - s3://your-bucket/config/version.json
   
   # Deploy CDK stack
   cd cdk
   npm run build
   cdk deploy FrontendStack
   ```

3. **Wait for propagation**: Changes may take 5-15 minutes to propagate to all CloudFront edge locations

### Rollback

To rollback to a previous version:

```bash
echo '{"version": "0.3.0"}' | aws s3 cp - s3://your-bucket/config/version.json
```

## Performance Considerations

### Caching Strategy

1. **CloudFront Caching**: Standard CDN caching for static assets
2. **No Runtime Processing**: No Lambda execution or S3 calls on each request
3. **Origin Path**: Direct serving from version-specific S3 folder

### Latency Impact

- No additional request latency (no Lambda@Edge execution)
- Standard CloudFront performance characteristics
- Edge cache invalidation required for immediate updates (optional)

## Monitoring and Troubleshooting

### CloudWatch Logs

CDK deployment logs are available in CloudWatch Logs and CDK CLI output.

Common log messages:
- `Using version from config: X.Y.Z` - Version read from config file
- `Using fallback version: X.Y.Z` - Config file not found, using default
- `✅ CDK deployment successful!` - Origin path updated successfully

### Common Issues

1. **Version not updating**
   - Check if version.json is properly formatted JSON
   - Verify CDK deployment completed successfully
   - Check CloudFormation stack status

2. **CDK Deployment Failures**
   - Review CDK CLI output for errors
   - Check AWS credentials and permissions
   - Verify CDK CLI is installed and up to date

3. **CloudFront Caching**
   - Create invalidation if immediate update needed
   - Wait 5-15 minutes for natural cache expiration
   - Check CloudFront distribution configuration

### Testing

Test the CDK stack configuration:

```bash
cd cdk
npm run build
npm test
```

Test version configuration:

```bash
./scripts/update-frontend-version.sh 1.0.0
```

## Security Considerations

### S3 Bucket Access

The S3 bucket uses Origin Access Control (OAC) for secure access from CloudFront. No public bucket policies are required.

### CDK Deployment Permissions

The deployment process requires:
- CloudFormation permissions for stack updates
- CloudFront permissions for distribution updates
- S3 permissions for bucket and object access

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

3. **Geographic Routing**
   ```json
   {
     "version": "1.0.0",
     "geographic": {
       "US": "1.0.0",
       "EU": "1.0.1"
     }
   }
   ```

## Cost Analysis

### Additional Costs

1. **CloudFormation Stack Updates**: No additional cost for updates
2. **CloudFront Distribution Updates**: No additional cost for configuration changes
3. **S3 Storage**: Minimal cost for version configuration file

### Cost Optimization

- No Lambda@Edge execution costs
- No additional S3 API calls during request processing
- Simplified architecture reduces operational overhead

## Migration from Lambda@Edge Version

To migrate from the previous Lambda@Edge system:

1. Deploy the updated CDK stack (this change)
2. Verify initial version.json matches current deployment
3. Test version change with update script
4. Remove any Lambda@Edge monitoring that's no longer needed

The migration removes the Lambda@Edge function and simplifies the architecture while maintaining version management capabilities.