# ECR Pull Through Cache Setup for GHCR

This project is configured to use AWS ECR pull through cache to automatically pull Docker images from GitHub Container Registry (GHCR) into your ECR registry for Lambda deployments.

## Prerequisites

- AWS CLI configured with appropriate permissions
- Administrator access to configure ECR pull through cache rules

## Setup Instructions

### 1. Configure ECR Pull Through Cache Rule

Create a pull through cache rule in your AWS account to automatically mirror GHCR images:

```bash
aws ecr put-replication-configuration \
    --replication-configuration '{
        "rules": [
            {
                "destinations": [
                    {
                        "region": "us-west-2",
                        "registryId": "159222827421"
                    }
                ],
                "repositoryFilters": [
                    {
                        "filter": "github/saebyn/glowing-telegram/*",
                        "filterType": "PREFIX_MATCH"
                    }
                ]
            }
        ]
    }'
```

Or use the AWS Console:

1. Navigate to ECR in AWS Console
2. Go to "Private registry" → "Pull through cache"
3. Click "Create rule"
4. Set upstream registry type: "GitHub Container Registry"
5. Set ECR repository prefix: `github`
6. This will automatically create repositories like `github/saebyn/glowing-telegram/ai-chat-lambda`

### 2. Initial Image Pull

The first time each service is deployed, ECR will automatically pull the image from GHCR and cache it locally. This may take a few extra minutes for the first deployment.

### 3. Verify Setup

You can verify the pull through cache is working by checking your ECR repositories:

```bash
aws ecr describe-repositories --region us-west-2 | grep "github/saebyn/glowing-telegram"
```

## How It Works

1. **Build and Push**: GitHub Actions builds Docker images and pushes them to GHCR only (`ghcr.io/saebyn/glowing-telegram/service-name:latest`)

2. **ECR Pull Through**: When AWS Lambda or ECS tries to pull an image, ECR automatically:
   - Checks if the image exists locally in ECR
   - If not found, pulls it from GHCR and caches it
   - Serves subsequent requests from the local ECR cache

3. **CDK Deployment**: The CDK stack references ECR repository names like `github/saebyn/glowing-telegram/service-name`, which correspond to the pull through cache repositories

## Repository Mapping

| Service | GHCR Repository | ECR Pull Through Repository |
|---------|----------------|------------------------------|
| AI Chat Lambda | `ghcr.io/saebyn/glowing-telegram/ai-chat-lambda` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/ai-chat-lambda` |
| Audio Transcriber | `ghcr.io/saebyn/glowing-telegram/audio-transcriber` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/audio-transcriber` |
| CRUD API | `ghcr.io/saebyn/glowing-telegram/crud-api` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/crud-api` |
| Media Lambda | `ghcr.io/saebyn/glowing-telegram/media-lambda` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/media-lambda` |
| Render Job | `ghcr.io/saebyn/glowing-telegram/render-job` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/render-job` |
| Summarize Transcription | `ghcr.io/saebyn/glowing-telegram/summarize-transcription` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/summarize-transcription` |
| Twitch Lambda | `ghcr.io/saebyn/glowing-telegram/twitch-lambda` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/twitch-lambda` |
| Upload Video | `ghcr.io/saebyn/glowing-telegram/upload-video` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/upload-video` |
| Video Ingestor | `ghcr.io/saebyn/glowing-telegram/video-ingestor` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/video-ingestor` |
| Youtube Lambda | `ghcr.io/saebyn/glowing-telegram/youtube-lambda` | `159222827421.dkr.ecr.us-west-2.amazonaws.com/github/saebyn/glowing-telegram/youtube-lambda` |

## Benefits

- **Simplified CI/CD**: Only need to push to GHCR, no ECR credentials required in GitHub Actions
- **Automatic Caching**: ECR automatically caches images locally for faster deployments
- **Cost Optimization**: Only pull from GHCR when needed, serve subsequent requests from ECR cache
- **Public Access**: GHCR images are publicly accessible for easier distribution
- **No Dual Push**: Eliminates the need to maintain two separate push processes

## Troubleshooting

- **First deployment takes longer**: This is expected as ECR pulls and caches the image from GHCR
- **Repository not found**: Ensure the pull through cache rule is configured correctly
- **Permission denied**: Verify your AWS credentials have ECR pull through cache permissions