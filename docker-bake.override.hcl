# Override file providing ECR tagging configuration
# This overrides the existing targets to use versioned tags

variable "IMAGE_TAG" {
  default = "latest"
}

target "ai_chat_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/ai-chat-lambda:${IMAGE_TAG}"
  ]
}

target "audio_transcriber" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/audio-transcription:${IMAGE_TAG}"
  ]
}

target "crud_api" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/crud-lambda:${IMAGE_TAG}"
  ]
}

target "embedding_service" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/embedding-service:${IMAGE_TAG}"
  ]
}

target "media_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/media-lambda:${IMAGE_TAG}"
  ]
}

target "render_job" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/render-job:${IMAGE_TAG}"
  ]
}

target "summarize_transcription" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/summarize-transcription-lambda:${IMAGE_TAG}"
  ]
}

target "twitch_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/twitch-lambda:${IMAGE_TAG}"
  ]
}

target "upload_video" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/upload-video:${IMAGE_TAG}"
  ]
}

target "video_ingestor" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/video-ingestor:${IMAGE_TAG}"
  ]
}

target "youtube_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/youtube-lambda:${IMAGE_TAG}"
  ]
}

target "youtube_uploader_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/youtube-uploader-lambda:${IMAGE_TAG}"
  ]
}

target "chat_processor_lambda" {
  tags = [
    "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/chat-processor-lambda:${IMAGE_TAG}"
  ]
}
