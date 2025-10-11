# glowing-telegram

A tool for managing stream recordings.

This is a tool for managing stream recordings, ingesting them into a database, providing a web interface for searching, analyzing, and passing them to a video processing pipeline.

[I'm developing this tool live on Twitch. Why not come check it out sometime?](https://twitch.tv/saebyn) I'm developing this tool to practice my Rust, as it's a bit rusty, and to automate some of the video processing tasks that I do manually by spending way more time programming than I would have spent doing the tasks manually.

## Features

1. Track locally recorded clips from a stream
1. Generate a set of "episodes" from the stream based on when the speaker is speaking
1. Episode transcription
1. Review interface for the transcriptions
1. Automatic summaries of the episode via text summarization provided by GPT-4
1. Capture chat messages with author and timestamp metadata from the stream
1. Flag areas of the video that are interesting
1. Generate a set of "highlights" from the stream based on the flagged areas
1. Generate chapter markers for the episode based on the flagged areas
1. Archive the stream videos to a cloud storage provider
1. Generate an OTIO file for for the stream video for use in a video editing tool

## Architecture

The tool is broken down into several repositories:

1. `glowing-telegram` - The backend for the tool (this repository)
1. [glowing-telegram-frontend](https:://github.com/saebyn/glowing-telegram-frontend) - The frontend for the tool
1. [glowing-telegram-video-editor](https://github.com/saebyn/glowing-telegram-video-editor) - A React component for reviewing stream videos and generating episodes

This repository contains these directories:

1. `ai_chat_lambda` - A lambda function for that wraps the OpenAI API for chat completion
1. `audio_transcriber` - An executable for transcribing audio files with OpenAI's Whisper Python library
1. `cdk` - An AWS CDK project for deploying the backend to AWS
1. `crud_api` - A lambda function for managing the CRUD operations for the DynamoDB tables
1. `docs` - Documentation for the project
1. `gt_ffmpeg` - A library for interacting with FFmpeg
1. `scripts` - Scripts for managing the project, migrating data from the old database, and other tasks
1. `summarize_transcription` - A lambda function for summarizing the transcriptions of the episodes using OpenAI's API
1. `twitch_bot` - An unfinished Twitch bot for interacting with Twitch chat and storing messages in the database, implemented in Elixir
1. `twitch_lambda` - A lambda function for ingesting authenticating with Twitch
1. `types` - Shared types for the project generated from the JSON schemas in the `docs` directory, also used by the frontend
1. `video_ingestor` - An executable for analyzing video files for silence detection, storing the speech audio track and keyframes of the video to S3, and storing the metadata in the database

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
- `ai-chat-lambda`
- `audio-transcription` 
- `crud-api`
- `embedding-service`
- `media-lambda`
- `render-job`
- `summarize-transcription`
- `twitch-lambda`
- `upload-video`
- `video-ingestor`
- `youtube-lambda`

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
    'crud-lambda',
    // ... other services ...
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

**Why all these files?**
- `docker-bake.hcl` - Defines how to build the image
- `docker-bake.override.hcl` - Enables versioned tagging for releases
- `cdk/lib/repoStack.ts` - Creates the ECR repository in AWS
- `scripts/push_image.sh` - Allows manual deployment of individual services

**Deployment Flow:**
1. Code pushed to GitHub triggers image builds
2. Images are pushed to ECR repositories (must exist!)
3. CDK updates infrastructure with new image versions

If you're interested in contributing beyond adding services, please reach out to me on [Twitch](https://twitch.tv/saebyn) or [Bluesky](https://bsky.app/profile/saebyn.bsky.social).
