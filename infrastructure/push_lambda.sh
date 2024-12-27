#!/bin/sh

# This script builds the Docker image, pushes it to ECR, and updates the Lambda function to use the new image.

# Usage: ./push.sh <SERVICE>
SERVICE=$1

# Check if the SERVICE argument is provided
if [ -z "$SERVICE" ]; then
  echo "Please provide the SERVICE argument"
  exit 1
fi

# Check if the SERVICE directory exists
if [ ! -d "$SERVICE" ]; then
  echo "The SERVICE directory does not exist"
  exit 1
fi

# Check if the Dockerfile exists
if [ ! -f "$SERVICE/Dockerfile" ]; then
  echo "The Dockerfile does not exist"
  exit 1
fi

# Map the SERVICE to the Lambda function name and ECR repository name
case $SERVICE in
  crud_api)
    FUNCTION_NAME="new-crud-lambda-ec79885"
    ECR_REPOSITORY="crud-lambda-ecr-b5e445c"
    ;;
  ai_chat_lambda)
    FUNCTION_NAME="new-ai-chat-lambda-0c271fa"
    ECR_REPOSITORY="ai-chat-lambda-ecr-781db3a"
    ;;
  summarize_transcription)
    FUNCTION_NAME="stream-ingestion-summarize_transcription_lambda-ac8a860"
    ECR_REPOSITORY="summarize_transcription"
    ;;
  *)
    echo "The SERVICE is not supported"
    exit 1
    ;;
esac

# Check if the AWS_ACCOUNT_ID environment variable is set
if [ -z "$AWS_ACCOUNT_ID" ]; then
  echo "The AWS_ACCOUNT_ID environment variable is not set"
  exit 1
fi

# Check if the AWS_REGION environment variable is set
if [ -z "$AWS_REGION" ]; then
  echo "The AWS_REGION environment variable is not set"
  exit 1
fi

# Generate the ECR URI
ECR_DOMAIN=$AWS_ACCOUNT_ID.dkr.ecr.$AWS_REGION.amazonaws.com

# Login to ECR
aws ecr get-login-password | docker login --username AWS --password-stdin $ECR_DOMAIN

# Build the Docker image
echo "Building Docker image for $SERVICE"
docker build -f $SERVICE/Dockerfile -t $SERVICE .

# Tag the image with the ECR repository URI
echo "Tagging Docker image for $SERVICE"
docker tag $SERVICE:latest $ECR_DOMAIN/$ECR_REPOSITORY:latest

# Push the image to ECR
echo "Pushing Docker image for $SERVICE"
docker push $ECR_DOMAIN/$ECR_REPOSITORY:latest

# Make the lambda function use the new image
echo "Updating Lambda function to use the new image"
aws lambda update-function-code \
    --function-name $FUNCTION_NAME\
    --image-uri $ECR_DOMAIN/$ECR_REPOSITORY:latest
