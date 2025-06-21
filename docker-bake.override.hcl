# Override file for GitHub Container Registry (GHCR) builds
# This extends the existing targets with GHCR tags

target "ai_chat_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/ai-chat-lambda:latest",
    "ghcr.io/saebyn/glowing-telegram/ai-chat-lambda:latest"
  ]
}

target "audio_transcriber" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/audio-transcription:latest",
    "ghcr.io/saebyn/glowing-telegram/audio-transcriber:latest"
  ]
}

target "crud_api" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/crud-lambda:latest",
    "ghcr.io/saebyn/glowing-telegram/crud-api:latest"
  ]
}

target "media_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/media-lambda:latest",
    "ghcr.io/saebyn/glowing-telegram/media-lambda:latest"
  ]
}

target "render_job" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/render-job:latest",
    "ghcr.io/saebyn/glowing-telegram/render-job:latest"
  ]
}

target "summarize_transcription" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/summarize-transcription-lambda:latest",
    "ghcr.io/saebyn/glowing-telegram/summarize-transcription:latest"
  ]
}

target "twitch_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/twitch-lambda:latest",
    "ghcr.io/saebyn/glowing-telegram/twitch-lambda:latest"
  ]
}

target "upload_video" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/upload-video:latest",
    "ghcr.io/saebyn/glowing-telegram/upload-video:latest"
  ]
}

target "video_ingestor" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/video-ingestor:latest",
    "ghcr.io/saebyn/glowing-telegram/video-ingestor:latest"
  ]
}

target "youtube_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/youtube-lambda:latest",
    "ghcr.io/saebyn/glowing-telegram/youtube-lambda:latest"
  ]
}