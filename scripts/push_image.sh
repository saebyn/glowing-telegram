#!/bin/sh

# This script builds the Docker image, pushes it to ECR, and updates the Lambda function to use the new image.

# Usage: ./push.sh <SERVICE>
SERVICE=$1

# Check if the SERVICE argument is provided
if [ -z "$SERVICE" ]; then
  echo "Please provide the SERVICE argument"
  exit 1
fi

# Map the SERVICE to the Lambda function name and ECR repository name
case $SERVICE in
  crud_api)
    FUNCTION_NAME="AppStack-APICrudLambda1ABF4DB4-Fx7l4uhYPntz"
    ECR_REPOSITORY="glowing-telegram/crud-lambda"
    ;;
  ai_chat_lambda)
    FUNCTION_NAME="AppStack-APIAiChatLambda6FCC65B9-cKJPcgIBdSRA"
    ECR_REPOSITORY="glowing-telegram/ai-chat-lambda"
    ;;
  summarize_transcription)
    FUNCTION_NAME="AppStack-StreamIngestionSummarizeTranscriptionLamb-DvNQhxeKUk43"
    ECR_REPOSITORY="glowing-telegram/summarize-transcription-lambda"
    ;;
  embedding_service)
    FUNCTION_NAME=""
    ECR_REPOSITORY="glowing-telegram/embedding-service"
    ;;
  audio_transcriber)
    FUNCTION_NAME=""
    ECR_REPOSITORY="glowing-telegram/audio-transcription"
    ;;
  video_ingestor)
    FUNCTION_NAME=""
    ECR_REPOSITORY="glowing-telegram/video-ingestor"
    ;;
  twitch_lambda)
    FUNCTION_NAME="AppStack-APITwitchLambda2D310BDC-j4oOL948PVWw"
    ECR_REPOSITORY="glowing-telegram/twitch-lambda"
    ;;
  youtube_lambda)
    FUNCTION_NAME="AppStack-APIYoutubeLambdaB9D8CD47-lNwxygsmjG5g"
    ECR_REPOSITORY="glowing-telegram/youtube-lambda"
    ;;
  media_lambda)
    FUNCTION_NAME="AppStack-MediaServePlaylistMediaLambda657BA445-dkfapvOB8vdT"
    ECR_REPOSITORY="glowing-telegram/media-lambda"
    ;;
  render_job)
    FUNCTION_NAME=""
    ECR_REPOSITORY="glowing-telegram/render-job"
    ;;
  upload_video)
    FUNCTION_NAME=""
    ECR_REPOSITORY="glowing-telegram/upload-video"
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

# Check that docker is up
if ! docker info > /dev/null 2>&1; then
  echo "Docker is not running"
  exit 1
fi

# Login to ECR
aws ecr get-login-password | docker login --username AWS --password-stdin $ECR_DOMAIN

# Build the Docker image
echo "Building Docker image for $SERVICE"
docker buildx bake $SERVICE

# Push the image to ECR
echo "Pushing Docker image for $SERVICE"
docker push $ECR_DOMAIN/$ECR_REPOSITORY:latest

if [ -z "$FUNCTION_NAME" ]; then
  exit 0
fi

# Make the lambda function use the new image
echo "Updating Lambda function to use the new image"
aws lambda update-function-code \
    --function-name $FUNCTION_NAME\
    --image-uri $ECR_DOMAIN/$ECR_REPOSITORY:latest
