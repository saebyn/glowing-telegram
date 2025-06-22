# Override file for GitHub Container Registry (GHCR) builds
# This overrides the existing targets to only use GHCR tags

target "ai_chat_lambda" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/ai-chat-lambda:latest"
  ]
}

target "audio_transcriber" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/audio-transcriber:latest"
  ]
}

target "crud_api" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/crud-api:latest"
  ]
}

target "media_lambda" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/media-lambda:latest"
  ]
}

target "render_job" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/render-job:latest"
  ]
}

target "summarize_transcription" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/summarize-transcription:latest"
  ]
}

target "twitch_lambda" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/twitch-lambda:latest"
  ]
}

target "upload_video" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/upload-video:latest"
  ]
}

target "video_ingestor" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/video-ingestor:latest"
  ]
}

target "youtube_lambda" {
  tags = [
    "ghcr.io/saebyn/glowing-telegram/youtube-lambda:latest"
  ]
}