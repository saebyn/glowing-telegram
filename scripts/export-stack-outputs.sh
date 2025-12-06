#!/bin/bash

# Script to export CDK stack outputs for frontend configuration
# Usage: ./scripts/export-stack-outputs.sh <environment>
# Example: ./scripts/export-stack-outputs.sh dev

set -e

ENVIRONMENT=${1:-production}
AWS_REGION=${AWS_REGION:-us-west-2}

echo "Exporting stack outputs for environment: ${ENVIRONMENT}"
echo "AWS Region: ${AWS_REGION}"
echo ""

# Determine stack names based on environment
if [ "$ENVIRONMENT" = "production" ]; then
    APP_STACK="AppStack"
    FRONTEND_STACK="FrontendStack"
    REPO_STACK="RepoStack"
else
    APP_STACK="AppStack-${ENVIRONMENT}"
    FRONTEND_STACK="FrontendStack-${ENVIRONMENT}"
    REPO_STACK="RepoStack-${ENVIRONMENT}"
fi

echo "Stack names:"
echo "  - ${APP_STACK}"
echo "  - ${FRONTEND_STACK}"
echo "  - ${REPO_STACK}"
echo ""

# Create output directory
OUTPUT_DIR="./frontend-config"
mkdir -p "$OUTPUT_DIR"

# Export AppStack outputs
echo "Exporting ${APP_STACK} outputs..."
aws cloudformation describe-stacks \
    --stack-name "$APP_STACK" \
    --region "$AWS_REGION" \
    --query 'Stacks[0].Outputs' \
    --output json > "$OUTPUT_DIR/${APP_STACK}-outputs.json" 2>/dev/null || {
    echo "Warning: Could not export ${APP_STACK} outputs (stack may not exist yet)"
}

# Export FrontendStack outputs
echo "Exporting ${FRONTEND_STACK} outputs..."
aws cloudformation describe-stacks \
    --stack-name "$FRONTEND_STACK" \
    --region "$AWS_REGION" \
    --query 'Stacks[0].Outputs' \
    --output json > "$OUTPUT_DIR/${FRONTEND_STACK}-outputs.json" 2>/dev/null || {
    echo "Warning: Could not export ${FRONTEND_STACK} outputs (stack may not exist yet)"
}

# Export RepoStack outputs
echo "Exporting ${REPO_STACK} outputs..."
aws cloudformation describe-stacks \
    --stack-name "$REPO_STACK" \
    --region "$AWS_REGION" \
    --query 'Stacks[0].Outputs' \
    --output json > "$OUTPUT_DIR/${REPO_STACK}-outputs.json" 2>/dev/null || {
    echo "Warning: Could not export ${REPO_STACK} outputs (stack may not exist yet)"
}

echo ""
echo "Outputs exported to: ${OUTPUT_DIR}/"
echo ""

# Extract key values for easy reference
echo "=== Key Configuration Values ==="
echo ""

if [ -f "$OUTPUT_DIR/${FRONTEND_STACK}-outputs.json" ]; then
    echo "Frontend Configuration:"
    DISTRIBUTION_DOMAIN=$(jq -r '.[] | select(.OutputKey=="CloudFrontDistributionDomainName") | .OutputValue' "$OUTPUT_DIR/${FRONTEND_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    BUCKET_NAME=$(jq -r '.[] | select(.OutputKey=="FrontendBucketName") | .OutputValue' "$OUTPUT_DIR/${FRONTEND_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    echo "  CloudFront Domain: ${DISTRIBUTION_DOMAIN}"
    echo "  S3 Bucket: ${BUCKET_NAME}"
    echo ""
fi

if [ -f "$OUTPUT_DIR/${REPO_STACK}-outputs.json" ]; then
    echo "GitHub Actions Configuration:"
    GITHUB_ROLE_ARN=$(jq -r '.[] | select(.OutputKey=="GithubActionRoleArn") | .OutputValue' "$OUTPUT_DIR/${REPO_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    echo "  IAM Role ARN: ${GITHUB_ROLE_ARN}"
    echo ""
fi

if [ -f "$OUTPUT_DIR/${APP_STACK}-outputs.json" ]; then
    echo "API Configuration:"
    API_URL=$(jq -r '.[] | select(.OutputKey=="ApiUrl") | .OutputValue' "$OUTPUT_DIR/${APP_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    WEBSOCKET_URL=$(jq -r '.[] | select(.OutputKey=="WebSocketApiUrl") | .OutputValue' "$OUTPUT_DIR/${APP_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    echo "  API Gateway URL: ${API_URL}"
    echo "  WebSocket URL: ${WEBSOCKET_URL}"
    echo ""
    
    echo "Cognito Configuration:"
    USER_POOL_ID=$(jq -r '.[] | select(.OutputKey=="UserPoolId") | .OutputValue' "$OUTPUT_DIR/${APP_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    USER_POOL_CLIENT_ID=$(jq -r '.[] | select(.OutputKey=="UserPoolClientId") | .OutputValue' "$OUTPUT_DIR/${APP_STACK}-outputs.json" 2>/dev/null || echo "N/A")
    echo "  User Pool ID: ${USER_POOL_ID}"
    echo "  User Pool Client ID: ${USER_POOL_CLIENT_ID}"
    echo ""
fi

echo "=== GitHub Environment Variables ==="
echo ""
echo "Add these to GitHub Environment '${ENVIRONMENT}' in glowing-telegram-frontend:"
echo ""
echo "Secrets:"
[ -f "$OUTPUT_DIR/${REPO_STACK}-outputs.json" ] && echo "  AWS_ROLE_ARN: ${GITHUB_ROLE_ARN}"
[ -f "$OUTPUT_DIR/${FRONTEND_STACK}-outputs.json" ] && echo "  FRONTEND_BUCKET: ${BUCKET_NAME}"
echo ""
echo "Variables:"
[ -f "$OUTPUT_DIR/${APP_STACK}-outputs.json" ] && {
    echo "  API_URL: ${API_URL}"
    echo "  WEBSOCKET_URL: ${WEBSOCKET_URL}"
    echo "  USER_POOL_ID: ${USER_POOL_ID}"
    echo "  USER_POOL_CLIENT_ID: ${USER_POOL_CLIENT_ID}"
}
[ -f "$OUTPUT_DIR/${FRONTEND_STACK}-outputs.json" ] && echo "  CLOUDFRONT_DOMAIN: ${DISTRIBUTION_DOMAIN}"
echo ""

echo "Done! Review the exported JSON files in ${OUTPUT_DIR}/ for complete details."
