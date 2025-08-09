#!/bin/bash

# Script to rollback frontend version to previous version
# Usage: ./rollback-frontend-version.sh [bucket-name]

set -euo pipefail

# Configuration
BUCKET_NAME="${1:-}"
CONFIG_FILE="config/version.json"

echo "Frontend Version Rollback Script"
echo "================================="

# Get bucket name if not provided
if [ -z "$BUCKET_NAME" ]; then
    echo "Getting frontend bucket name from CDK outputs..."
    BUCKET_NAME=$(aws cloudformation describe-stacks \
        --stack-name FrontendStack \
        --query 'Stacks[0].Outputs[?OutputKey==`FrontendAssetBucketName`].OutputValue' \
        --output text 2>/dev/null || echo "")
    
    if [ -z "$BUCKET_NAME" ]; then
        echo "Error: Could not determine bucket name automatically"
        echo "Please provide bucket name as argument"
        echo "Usage: $0 <bucket-name>"
        exit 1
    fi
fi

echo "Target bucket: $BUCKET_NAME"

# Get current version configuration
echo "Fetching current version configuration..."
CURRENT_CONFIG=$(aws s3 cp "s3://$BUCKET_NAME/$CONFIG_FILE" - 2>/dev/null || echo '{}')

if [ "$CURRENT_CONFIG" = '{}' ]; then
    echo "Error: No version configuration found"
    exit 1
fi

CURRENT_VERSION=$(echo "$CURRENT_CONFIG" | jq -r '.version // "unknown"')
ROLLBACK_VERSION=$(echo "$CURRENT_CONFIG" | jq -r '.rollbackVersion // null')

echo "Current version: $CURRENT_VERSION"

if [ "$ROLLBACK_VERSION" = "null" ] || [ -z "$ROLLBACK_VERSION" ]; then
    echo "Error: No rollback version specified in configuration"
    echo "Current configuration:"
    echo "$CURRENT_CONFIG" | jq .
    exit 1
fi

echo "Rollback version: $ROLLBACK_VERSION"

# Confirm rollback
echo ""
echo "This will rollback from version $CURRENT_VERSION to $ROLLBACK_VERSION"
read -p "Proceed with rollback? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Rollback cancelled"
    exit 0
fi

# Create rollback configuration
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
ROLLBACK_CONFIG=$(cat <<EOF
{
  "version": "$ROLLBACK_VERSION",
  "description": "Frontend version rolled back via script",
  "lastUpdated": "$TIMESTAMP",
  "rollbackVersion": "$CURRENT_VERSION",
  "metadata": {
    "deployedBy": "$(whoami)",
    "environment": "production",
    "action": "rollback",
    "previousVersion": "$CURRENT_VERSION"
  }
}
EOF
)

echo "Rollback configuration:"
echo "$ROLLBACK_CONFIG" | jq .

# Upload rollback configuration
echo "Uploading rollback configuration..."
echo "$ROLLBACK_CONFIG" | aws s3 cp - "s3://$BUCKET_NAME/$CONFIG_FILE" \
    --content-type "application/json" \
    --metadata "version=$ROLLBACK_VERSION,rolled-back-by=$(whoami),action=rollback"

# Verify rollback
echo "Verifying rollback..."
UPLOADED_CONFIG=$(aws s3 cp "s3://$BUCKET_NAME/$CONFIG_FILE" -)
UPLOADED_VERSION=$(echo "$UPLOADED_CONFIG" | jq -r '.version')

if [ "$UPLOADED_VERSION" = "$ROLLBACK_VERSION" ]; then
    echo "✅ Rollback successful!"
    echo "Rolled back to version: $ROLLBACK_VERSION"
    echo "Changes will take effect within 60 seconds due to Lambda@Edge caching"
else
    echo "❌ Rollback failed!"
    echo "Expected: $ROLLBACK_VERSION"
    echo "Got: $UPLOADED_VERSION"
    exit 1
fi

echo "Done! 🔄"