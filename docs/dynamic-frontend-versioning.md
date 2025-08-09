# Dynamic Frontend Version Selection

This document describes the dynamic frontend version selection feature implemented using CloudFront and Lambda@Edge.

## Overview

The dynamic frontend version selection feature allows you to change the frontend version served by CloudFront without redeploying the CDK stack. This is achieved through:

1. A Lambda@Edge function that reads version configuration from S3
2. Dynamic path rewriting based on the version configuration
3. In-memory caching for performance optimization
4. Public read access for the version configuration file

## Architecture

```
CloudFront Request → Lambda@Edge (Viewer Request) → S3 Origin
                           ↓
                    Version Config (S3)
                    config/version.json
```

### Components

1. **Lambda@Edge Function**: `version-selector`
   - Triggers on viewer requests to CloudFront
   - Reads version configuration from S3
   - Rewrites request URI to include version path
   - Implements 60-second in-memory caching

2. **Version Configuration File**: `config/version.json`
   - Stored in the frontend assets S3 bucket
   - Publicly readable for Lambda@Edge access
   - Contains current version and metadata

3. **CloudFront Distribution**
   - Configured without hardcoded origin path
   - Uses Lambda@Edge for dynamic routing
   - Maintains CDN caching benefits

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
   aws s3 cp updated-version.json s3://your-bucket/config/version.json
   ```

2. **Or use AWS CLI to update directly**:
   ```bash
   echo '{"version": "1.2.0"}' | aws s3 cp - s3://your-bucket/config/version.json
   ```

3. **Wait for cache expiration**: Changes take effect within 60 seconds due to Lambda@Edge caching

### Rollback

To rollback to a previous version:

```bash
echo '{"version": "0.3.0"}' | aws s3 cp - s3://your-bucket/config/version.json
```

## Performance Considerations

### Caching Strategy

1. **Lambda@Edge Caching**: 60-second TTL for version config
2. **CloudFront Caching**: Standard CDN caching for static assets
3. **S3 Access**: Minimized through in-memory caching

### Latency Impact

- First request after cache expiry: +50-100ms (S3 read)
- Subsequent requests: No additional latency
- Failed S3 requests: Fallback to cached version or pass-through

## Monitoring and Troubleshooting

### CloudWatch Logs

Lambda@Edge logs are available in CloudWatch Logs in the region where the function executes (typically us-east-1 for global distributions).

Common log messages:
- `Fetched and cached new version: X.Y.Z` - Successful version update
- `Using cached version: X.Y.Z` - Cache hit
- `Error fetching version from S3` - S3 access error
- `No version found, proceeding with original request` - Fallback behavior

### Common Issues

1. **Version not updating**
   - Check if version.json is properly formatted JSON
   - Verify S3 bucket permissions
   - Wait 60 seconds for cache expiration

2. **S3 Access Denied**
   - Ensure version.json has public read permissions
   - Check Lambda@Edge execution role permissions

3. **Lambda@Edge Errors**
   - Review CloudWatch Logs in us-east-1
   - Check function timeout settings
   - Verify function code deployment

### Testing

Test the Lambda@Edge function locally:

```bash
cd cdk/lambda/version-selector
npm test
```

Test version configuration:

```bash
curl -s https://your-cloudfront-domain.cloudfront.net/ -v
# Check X-Forwarded-For headers and response paths
```

## Security Considerations

### S3 Bucket Policy

The version configuration file requires public read access:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::your-bucket/config/version.json"
    }
  ]
}
```

### Lambda@Edge Permissions

The Lambda@Edge function requires:
- Basic execution role for Lambda
- S3 GetObject permission for the version config file
- CloudWatch Logs permissions for debugging

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

1. **Lambda@Edge Invocations**: $0.0000006 per request
2. **S3 GET Requests**: $0.0004 per 1,000 requests (cached, so minimal)
3. **CloudWatch Logs**: Standard logging costs

### Cost Optimization

- 60-second caching reduces S3 requests by ~99%
- Single version config file minimizes S3 storage costs
- Lambda@Edge function optimized for minimal execution time

## Migration from Static Version

To migrate from the previous static version system:

1. Deploy the updated CDK stack
2. Upload initial version.json with current version
3. Verify functionality with a test version change
4. Update deployment scripts to modify version.json instead of redeploying CDK

The migration is backward compatible - if version.json is missing or invalid, requests pass through unchanged.