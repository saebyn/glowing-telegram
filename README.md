# glowing-telegram

A comprehensive stream and video management platform with real-time capabilities.

This is a full-featured platform for managing live streams and recordings, featuring real-time Twitch chat integration, AI-powered transcription and analysis, semantic search, stream widgets for OBS, and automated video processing pipelines. The system ingests stream recordings into a database, provides a web interface for searching and analyzing content, and includes WebSocket APIs for real-time UI updates.

[I'm developing this tool live on Twitch. Why not come check it out sometime?](https://twitch.tv/saebyn) I'm developing this tool to practice my Rust skills and to automate video processing tasks that would take hours to do manually. The project has evolved from a simple recording manager into a comprehensive streaming platform with AI integration, real-time features, and semantic search capabilities.

## Features

### Stream Management
1. Track locally recorded clips from a stream with comprehensive metadata
1. Generate "episodes" from streams based on speech detection and silence analysis
1. Archive stream videos to cloud storage (AWS S3)
1. Render episodes directly from selected video segments with automated processing

### AI-Powered Analysis
1. **Audio Transcription** - Automated transcription using HuggingFace Whisper models with silence detection
1. **Semantic Search** - Vector embeddings with pgvector for intelligent content discovery
1. **AI Summaries** - Automatic episode summaries via GPT-4
1. **Embedding Service** - Aurora Serverless v2 with pgvector for similarity search

### Real-Time Features
1. **Twitch Chat Integration** - Capture chat messages with EventSub webhooks
1. **WebSocket API** - Real-time UI updates and bidirectional communication
1. **Stream Widgets** - Dynamic OBS browser sources (countdown timers, overlays, etc.)
1. **Live Processing** - Real-time chat processing and message storage

### Video Processing
1. Review interface for transcriptions and episode editing
1. Select and arrange video segments for episode creation
1. In-app video rendering with customizable CutLists
1. Automatic chapter marker generation
1. Silence detection for improved transcription accuracy
1. Audio remixing capabilities with track selection and filtering

### Platform Integration
1. **YouTube Upload** - Automated upload of rendered episodes with metadata
1. **Twitch EventSub** - Webhook-based event processing
1. **CloudFront CDN** - Dynamic content delivery with versioning

## Architecture

### Multi-Repository Structure

The platform is divided into specialized repositories:

1. `glowing-telegram` - Backend services and infrastructure (this repository)
1. [glowing-telegram-frontend](https://github.com/saebyn/glowing-telegram-frontend) - React-based web interface
1. [glowing-telegram-video-editor](https://github.com/saebyn/glowing-telegram-video-editor) - React component for video review and episode generation

### System Architecture

**API Layer:**
- **HTTP API** - RESTful CRUD operations with Cognito authentication
- **WebSocket API** - Real-time bidirectional communication
- **EventSub Webhooks** - Twitch event processing

**Processing Layer:**
- **Lambda Functions** - Serverless API handlers and event processors
- **AWS Batch** - GPU-accelerated transcription and video processing
- **SQS Queues** - Asynchronous message processing
- **DynamoDB Streams** - Change data capture for real-time updates

**Storage Layer:**
- **DynamoDB** - Primary datastore for streams, episodes, widgets, chat
- **Aurora PostgreSQL** - Vector embeddings with pgvector
- **S3** - Video files, audio tracks, transcripts, and assets
- **EFS** - ML model caching for HuggingFace transformers

**Delivery Layer:**
- **CloudFront** - CDN for frontend assets with dynamic origin updates
- **API Gateway** - HTTP and WebSocket endpoint management

### Recent Architectural Improvements

**Performance Optimizations:**
- CloudFront invalidation moved to run immediately after video ingestion (not after transcription)
- Silence detection integration reduces transcription time and improves accuracy
- EFS model caching eliminates repeated downloads (~3GB per job)
- DynamoDB streams trigger real-time WebSocket broadcasts

**Reliability Enhancements:**
- 10-minute timeout protection for Whisper processes
- 1KB minimum audio size threshold to skip empty files
- Consistent CloudWatch logging with `/glowing-telegram/*` prefix
- HMAC signature verification for Twitch EventSub webhooks
- User-scoped CRUD API with automatic user_id injection

**Scalability Improvements:**
- Aurora Serverless v2 auto-scales from 0.5 to 1 ACU
- AWS Batch with spot instances for cost-effective GPU compute
- SQS-based chat processing decouples ingestion from storage
- WebSocket connection pooling for real-time features

This repository contains these directories:

### Lambda Functions (Containerized)
1. `ai_chat_lambda` - OpenAI API wrapper for chat completion
1. `chat_processor_lambda` - Processes Twitch chat messages from SQS queue
1. `crud_api` - RESTful API for DynamoDB CRUD operations with user-scoped access control
1. `media_lambda` - Handles media-related operations
1. `summarize_transcription` - Episode summarization using OpenAI's API
1. `twitch_lambda` - Twitch authentication and EventSub webhook handling
1. `websocket_lambda` - WebSocket API for real-time features (Docker-based)
1. `youtube_uploader_lambda` - YouTube upload automation (Python/Docker)

### Batch Processing Services
1. `audio_transcriber` - Transcribes audio using HuggingFace Whisper transformers with EFS model caching
1. `embedding_service` - Generates vector embeddings and stores them in Aurora PostgreSQL with pgvector
1. `render_job` - Video rendering pipeline
1. `upload_video` - Video upload processing
1. `video_ingestor` - Analyzes videos for silence detection, extracts audio tracks and keyframes

### Shared Libraries
1. `gt_app` - Common application utilities
1. `gt_axum` - Shared Axum web framework components
1. `gt_ffmpeg` - FFmpeg interaction library
1. `gt_secrets` - AWS Secrets Manager integration
1. `types` - Shared types generated from JSON schemas (used by both backend and frontend)

### Infrastructure & Tooling
1. `cdk` - AWS CDK infrastructure as code (TypeScript)
1. `docs` - JSON schemas, ER diagrams, and workflow documentation
1. `scripts` - Deployment scripts, data migration tools, S3 import utilities


## Key Technologies

### Infrastructure
- **AWS Lambda** - Serverless compute for API handlers and processing
- **AWS Batch** - GPU-accelerated transcription and video processing
- **Aurora Serverless v2** - PostgreSQL with pgvector for semantic search
- **DynamoDB** - NoSQL database for streams, episodes, chat messages, and widgets
- **S3** - Object storage for videos, audio, and assets
- **CloudFront** - CDN with dynamic origin updates
- **EFS** - Model caching for HuggingFace transformers
- **API Gateway** - HTTP and WebSocket APIs
- **EventBridge** - Event-driven processing

### AI & ML
- **HuggingFace Transformers** - Whisper large-v3 for audio transcription
- **OpenAI GPT-4** - Episode summarization and chat completion
- **OpenAI Embeddings** - text-embedding-3-small for semantic search
- **pgvector** - Vector similarity search in PostgreSQL

### Development Stack
- **Rust** - Primary backend language for performance-critical services
- **TypeScript** - CDK infrastructure and type definitions
- **Python** - Lambda functions for YouTube and media operations
- **Docker** - Containerized deployments for all services

## Database Schema

### DynamoDB Tables
- **streams** - Stream metadata and configurations
- **video_clips** - Individual video segments with timestamps
- **episodes** - Generated episodes with transcriptions and summaries
- **projects** - Project groupings for content organization
- **chat_messages** - Twitch chat messages with TTL (30 days)
- **stream_widgets** - Widget configurations and state (with GSIs for user_id, access_token, type, active)

### Aurora PostgreSQL
- **embeddings** - Vector embeddings with pgvector extension for semantic search

## Real-Time Features

### WebSocket API
The WebSocket API provides bidirectional communication for real-time UI updates.

**Authentication Methods:**
- **Cognito JWT** - Full access to all user's widgets, can execute actions
- **Widget Token** - Read-only access to single widget (for OBS browser sources)

**Message Types:**
- `WIDGET_SUBSCRIBE` - Subscribe to widget updates
- `WIDGET_UNSUBSCRIBE` - Unsubscribe from widget
- `WIDGET_ACTION` - Execute widget action (authenticated users only)
- `WIDGET_INITIAL_STATE` - Initial widget state on subscription
- `WIDGET_CONFIG_UPDATE` - Configuration change broadcast
- `WIDGET_STATE_UPDATE` - State change broadcast

### Stream Widgets
Stream widgets are synchronized UI components for OBS browser sources and web interfaces.

**Widget Types:**
- Countdown timers
- Text overlays
- Custom interactive elements

**Widget Configuration:**
```json
{
  "id": "uuid",
  "title": "Countdown Timer",
  "type": "countdown",
  "access_token": "uuid-for-obs-auth",
  "config": {
    "duration": 300,
    "text": "Starting soon",
    "title": "Stream Starting"
  },
  "state": {
    "duration_left": 300,
    "enabled": false,
    "last_tick_timestamp": "2025-11-22T10:00:00Z"
  }
}
```

**Usage in OBS:**
1. Create widget via web interface
2. Copy widget access URL (includes token)
3. Add browser source in OBS
4. Widget updates in real-time via WebSocket

### Twitch Chat Integration
Real-time chat message capture using Twitch EventSub webhooks.

**Features:**
- EventSub webhook subscription management
- Message validation with HMAC signature verification
- SQS-based message processing pipeline
- Chat message storage with 30-day TTL
- Automatic subscription status tracking

**Architecture:**
1. Twitch sends EventSub webhook to API Gateway
2. `twitch_lambda` validates signature and queues to SQS
3. `chat_processor_lambda` processes messages and stores in DynamoDB
4. Messages expire after 30 days via DynamoDB TTL

## Development

### Deployment

The project uses automated deployment via GitHub Actions for production releases. When you publish a release on GitHub, the system automatically:

1. **Builds and pushes Docker images** to Amazon ECR with the release tag
2. **Deploys the CDK application** using the newly built images

#### Production Deployment Process

To deploy to production:

1. **Create a release on GitHub:**
   - Go to the GitHub repository
   - Click "Releases" → "Create a new release"
   - Create a new tag (e.g., `v1.2.3`)
   - Publish the release

2. **Automated deployment happens:**
   - GitHub Actions triggers the `docker.yml` workflow
   - Docker images are built and pushed to ECR with the release tag
   - CDK deployment automatically updates infrastructure with the new image version
   - All services are updated to use the new images

#### Release-Based Deployment Details

The automated deployment process:
- **Trigger:** GitHub release events (when published)
- **Registry:** Amazon ECR (159222827421.dkr.ecr.us-west-2.amazonaws.com)
- **Tagging:** Uses the git tag from the release (e.g., `v1.2.3`)
- **Deployment:** CDK automatically deploys with `IMAGE_VERSION` set to the release tag

### Docker Images

#### Available Services

All services are built as container images:
- `ai-chat-lambda` - OpenAI chat completion wrapper
- `audio-transcription` - HuggingFace Whisper-based transcription
- `chat-processor-lambda` - Twitch chat message processing
- `crud-api` - DynamoDB CRUD API with user scoping
- `embedding-service` - Vector embeddings with Aurora/pgvector
- `media-lambda` - Media operations handler
- `render-job` - Video rendering pipeline
- `summarize-transcription` - AI-powered summarization
- `twitch-lambda` - Twitch EventSub integration
- `upload-video` - Video upload processing
- `video-ingestor` - Video analysis and silence detection
- `websocket-lambda` - WebSocket API for real-time features
- `youtube-lambda` - YouTube upload automation

#### Local Development

To build locally:
```bash
# Build all images with latest tag
docker buildx bake -f docker-bake.hcl all

# Build all images with custom version
IMAGE_TAG=v1.2.3 docker buildx bake -f docker-bake.hcl -f docker-bake.override.hcl all

# Build a specific image
docker buildx bake -f docker-bake.hcl crud_api
```

#### Manual CDK Deployment

For development or manual deployment, the CDK can be deployed with a specific image version:
```bash
cd cdk

# Install dependencies
npm install

# Deploy with specific image version
IMAGE_VERSION=v1.2.3 npm run cdk deploy

# Deploy with latest (default)
npm run cdk deploy
```

**Note:** Production deployments should use the automated GitHub Actions workflow triggered by releases rather than manual CDK deployment.

### Testing

#### Integration Tests

The project includes integration tests for various services. Use the `run_integration_tests.sh` script in the root directory to run tests for any service:

```bash
# Run integration tests for audio_transcriber
./run_integration_tests.sh audio_transcriber

# Run tests with verbose output
./run_integration_tests.sh audio_transcriber --verbose

# Build Docker image and run tests
./run_integration_tests.sh audio_transcriber --build

# Run tests without cleanup (for debugging)
./run_integration_tests.sh audio_transcriber --no-cleanup

# Run tests for other services
./run_integration_tests.sh video_ingestor
./run_integration_tests.sh embedding_service
```

The script automatically detects the service type (Rust, Node.js, Python) and runs the appropriate test commands. It also handles Docker image building and provides extensive configuration options.

For more information about available options:
```bash
./run_integration_tests.sh --help
```

#### Unit Tests

For Rust services, run unit tests with:
```bash
# Run all tests in workspace
cargo test --workspace

# Run tests for specific service
cd <service_directory>
cargo test
```

### Audio Transcription

The `audio_transcriber` service uses HuggingFace Whisper transformers for speech-to-text.

**Model:** `openai/whisper-large-v3`

**Features:**
- Silence detection integration to skip non-speech segments
- 1KB minimum audio size threshold
- 10-minute timeout protection
- EFS-based model caching across AWS Batch jobs
- GPU acceleration (g4dn instances)

**EFS Model Caching:**

Models are cached on EFS to avoid downloading on every job:
- **First run:** Downloads model (~3GB), takes ~5-10 minutes
- **Subsequent runs:** Uses cached model, much faster
- **Cache location:** `/mnt/efs/huggingface/` (mounted via EFS)
- **Environment:** `HF_HOME=/mnt/efs/huggingface`

**Troubleshooting:**

*Job hangs during model download:*
```bash
# Check EFS mount in AWS Console
# Verify mount target exists in job's subnet
# Check security group allows NFS (port 2049)
```

*Corrupted model cache:*
```bash
# Delete cached model via AWS EFS Console or:
# 1. Start EC2 instance in same VPC
# 2. Mount EFS: sudo mount -t nfs4 <efs-dns>:/ /mnt/efs
# 3. Remove model: rm -rf /mnt/efs/huggingface/hub/models--openai--whisper-large-v3
```

*Force model re-download:*
```bash
# Set environment variable in batch job definition:
# HF_HUB_OFFLINE=0
```

### Semantic Search with Embeddings

The `embedding_service` generates vector embeddings for semantic search capabilities.

**Infrastructure:**
- Aurora Serverless v2 PostgreSQL with pgvector extension
- Minimum capacity: 0.5 ACU, Maximum: 4 ACU
- Automatic scaling based on load
- VPC-isolated database cluster

**Features:**
- Text embedding generation using OpenAI text-embedding-3-small
- Vector similarity search with pgvector
- Automatic embedding updates via DynamoDB streams
- Integration tests with testcontainers

**Usage:**
1. Episodes/transcripts automatically generate embeddings
2. Embeddings stored in Aurora with pgvector
3. Search via vector similarity queries
4. Results ranked by cosine similarity

### CloudWatch Logging

All logs use consistent naming conventions:
- `/glowing-telegram/lambda/*` - Lambda functions
- `/glowing-telegram/apigateway/*` - API Gateway access logs
- `/glowing-telegram/stepfunctions/*` - Step Functions
- `/glowing-telegram/batch/*` - Batch jobs

**Retention:** 1 week for all log groups
**Removal Policy:** Destroy on stack deletion

### Environment Variables

Services are configured via environment variables injected by CDK:

**Common Variables:**
- `AWS_REGION` - AWS region (us-west-2)
- `DYNAMODB_TABLE` - Main DynamoDB table name
- `INPUT_BUCKET` - S3 bucket for input files
- `OUTPUT_BUCKET` - S3 bucket for processed output

**Service-Specific:**

*audio_transcriber:*
- `HF_HOME=/mnt/efs/huggingface` - HuggingFace model cache location
- `DEVICE=cuda` - Use GPU acceleration (auto-detected)

*embedding_service:*
- `DATABASE_URL` - Aurora PostgreSQL connection string
- `OPENAI_API_KEY` - Retrieved from AWS Secrets Manager

*twitch_lambda:*
- `EVENTSUB_SECRET` - Twitch EventSub webhook secret
- `CHAT_QUEUE_URL` - SQS queue for chat messages

*websocket_lambda:*
- `STREAM_WIDGETS_TABLE` - DynamoDB table for widgets
- `CONNECTIONS_TABLE` - WebSocket connection tracking

*youtube_uploader_lambda:*
- `YOUTUBE_SECRETS_BASE_PATH` - Base path for user YouTube credentials in Secrets Manager

### Configuration Files

**CDK Configuration:**
- `cdk/config/version.json` - Frontend version for CloudFront
- `cdk/cdk.context.json` - CDK context values

**Type Definitions:**
- `types/src/types.rs` - Rust types from JSON schemas
- `types/src/types.ts` - TypeScript types from JSON schemas
- Generated via `./types/import.sh`

### Troubleshooting

#### EFS Issues

*Mount target not found:*
- Verify EFS mount target exists in job's availability zone
- Check VPC subnet configuration
- Ensure security groups allow NFS traffic

*Permission denied on EFS:*
- Check EFS access point permissions
- Verify IAM role has required EFS permissions
- Ensure POSIX permissions are correctly set

#### Aurora Serverless

*Connection timeout:*
- Verify security group allows PostgreSQL (port 5432)
- Check VPC configuration and routing
- Ensure Lambda/Batch is in same VPC as Aurora

*Cold start issues:*
- Aurora Serverless v2 can pause when idle
- First query may take 10-30 seconds
- Consider setting minimum capacity higher for production

#### WebSocket API

*Authentication failures:*
- Verify Cognito JWT is valid and not expired
- Check widget access token matches database
- Ensure authorizer Lambda has proper permissions

*Messages not received:*
- Check CloudWatch logs for WebSocket Lambda
- Verify DynamoDB stream is enabled and connected
- Check connection ID is still valid (connections expire)

### Contributing

#### Adding a New Container Service

When adding a new container-based service to the project, you must update all of the following files to ensure proper deployment:

1. **`Dockerfile`** - Add a new build stage for your service
2. **`docker-bake.hcl`** - Add a target for your service in the appropriate batch group
3. **`docker-bake.override.hcl`** - Add a target with `${IMAGE_TAG}` variable for release tagging
4. **`cdk/lib/repoStack.ts`** - Add the repository name to the `names` array in `RepoConstruct`
5. **`scripts/push_image.sh`** - Add a case statement for individual deployments (if needed)
6. **`README.md`** - Add the service to the "Available Services" list

**Example: Adding a service called `my-service`**

```typescript
// cdk/lib/repoStack.ts
new RepoConstruct(this, 'RepoConstruct', {
  namespace: 'glowing-telegram',
  names: [
    'ai-chat-lambda',
    'audio-transcription',
    'chat-processor-lambda',
    'crud-api',
    'embedding-service',
    'media-lambda',
    'render-job',
    'summarize-transcription',
    'twitch-lambda',
    'upload-video',
    'video-ingestor',
    'websocket-lambda',
    'youtube-lambda',
    'my-service',  // Add your service here
  ],
});
```

```hcl
# docker-bake.hcl
target "my_service" {
  dockerfile = "Dockerfile"
  context = "."
  target = "my_service"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/my-service:latest"]
}
```

```hcl
# docker-bake.override.hcl
target "my_service" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/my-service:${IMAGE_TAG}"
  ]
}
```

```bash
# scripts/push_image.sh
case $SERVICE in
  # ... other services ...
  my-service)
    docker buildx bake -f docker-bake.hcl -f docker-bake.override.hcl my_service --push
    ;;
esac
```

**Why all these files?**
- `Dockerfile` - Multi-stage build definition for your service
- `docker-bake.hcl` - Defines how to build the image locally
- `docker-bake.override.hcl` - Enables versioned tagging for releases
- `cdk/lib/repoStack.ts` - Creates the ECR repository in AWS
- `scripts/push_image.sh` - Allows manual deployment of individual services
- `README.md` - Documents the service in "Available Services" list

**Deployment Flow:**
1. Release published on GitHub triggers automated workflow
2. Docker images built and pushed to ECR with release tag
3. CDK deployment updates infrastructure with new image versions
4. Services automatically updated with new images

**Important Notes:**
- All Lambda functions must use containerized deployments
- Python lambdas should use the Python 3.12 runtime base
- Rust services compile in Docker with cargo-lambda
- GPU services (transcription) use g4dn instances in AWS Batch

If you're interested in contributing beyond adding services, please reach out to me on [Twitch](https://twitch.tv/saebyn) or [Bluesky](https://bsky.app/profile/saebyn.bsky.social).

## Version History

For a complete version history, see [Releases](https://github.com/saebyn/glowing-telegram/releases).
