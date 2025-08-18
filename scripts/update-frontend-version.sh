#!/bin/bash

# Script to update frontend version dynamically
# Usage: ./update-frontend-version.sh <new-version> [bucket-name]

set -euo pipefail

# Configuration
NEW_VERSION="$1"
BUCKET_NAME="${2:-}"
CONFIG_FILE="config/version.json"

# Validate input
if [ -z "$NEW_VERSION" ]; then
    echo "Error: Version number is required"
    echo "Usage: $0 <new-version> [bucket-name]"
    echo "Example: $0 1.2.3"
    exit 1
fi

# Validate version format (basic semver check)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][a-zA-Z0-9]+)*$ ]]; then
    echo "Warning: Version '$NEW_VERSION' doesn't follow semver format (x.y.z)"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Get bucket name if not provided
if [ -z "$BUCKET_NAME" ]; then
    echo "Getting frontend bucket name from CDK outputs..."
    BUCKET_NAME=$(aws cloudformation describe-stacks \
        --stack-name FrontendStack \
        --query 'Stacks[0].Outputs[?OutputKey==`FrontendAssetBucketName`].OutputValue' \
        --output text 2>/dev/null || echo "")
    
    if [ -z "$BUCKET_NAME" ]; then
        echo "Error: Could not determine bucket name automatically"
        echo "Please provide bucket name as second argument"
        echo "Usage: $0 <new-version> <bucket-name>"
        exit 1
    fi
fi

echo "Updating frontend version to: $NEW_VERSION"
echo "Target bucket: $BUCKET_NAME"

# Get current version for backup
echo "Fetching current version configuration..."
CURRENT_CONFIG=$(aws s3 cp "s3://$BUCKET_NAME/$CONFIG_FILE" - 2>/dev/null || echo '{}')
CURRENT_VERSION=$(echo "$CURRENT_CONFIG" | jq -r '.version // "unknown"')

echo "Current version: $CURRENT_VERSION"

# Create new version configuration
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
NEW_CONFIG=$(cat <<EOF
{
  "version": "$NEW_VERSION",
  "description": "Frontend version updated via script",
  "lastUpdated": "$TIMESTAMP",
  "rollbackVersion": "$CURRENT_VERSION",
  "metadata": {
    "deployedBy": "$(whoami)",
    "environment": "production",
    "previousVersion": "$CURRENT_VERSION"
  }
}
EOF
)

echo "New configuration:"
echo "$NEW_CONFIG" | jq .

# Confirm update
read -p "Proceed with version update? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Update cancelled"
    exit 0
fi

# Upload new configuration
echo "Uploading new version configuration..."
echo "$NEW_CONFIG" | aws s3 cp - "s3://$BUCKET_NAME/$CONFIG_FILE" \
    --content-type "application/json" \
    --metadata "version=$NEW_VERSION,updated-by=$(whoami)"

# Verify upload
echo "Verifying upload..."
UPLOADED_CONFIG=$(aws s3 cp "s3://$BUCKET_NAME/$CONFIG_FILE" -)
UPLOADED_VERSION=$(echo "$UPLOADED_CONFIG" | jq -r '.version')

if [ "$UPLOADED_VERSION" = "$NEW_VERSION" ]; then
    echo "✅ Version update successful!"
    echo "New version: $NEW_VERSION"
    echo "CloudFront distribution will be automatically updated via Lambda function..."
    echo "⏳ Update may take 1-2 minutes to propagate through CloudFront"
else
    echo "❌ Version update failed!"
    echo "Expected: $NEW_VERSION"
    echo "Got: $UPLOADED_VERSION"
    exit 1
fi

# Optional: Test the update
read -p "Test the new version? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Get CloudFront domain
    CF_DOMAIN=$(aws cloudformation describe-stacks \
        --stack-name FrontendStack \
        --query 'Stacks[0].Outputs[?OutputKey==`CloudFrontDomainName`].OutputValue' \
        --output text 2>/dev/null || echo "")
    
    if [ -n "$CF_DOMAIN" ]; then
        echo "Testing CloudFront distribution: https://$CF_DOMAIN"
        echo "Note: CloudFront edge caches may take 5-15 minutes to update"
        
        # Simple test request
        HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "https://$CF_DOMAIN/" || echo "000")
        if [ "$HTTP_STATUS" = "200" ]; then
            echo "✅ Frontend is accessible"
        else
            echo "⚠️ Frontend returned HTTP $HTTP_STATUS"
        fi
    else
        echo "Could not determine CloudFront domain for testing"
    fi
fi

echo "Done! 🎉"