# Build all images with `docker buildx bake all`
# Build a specific image with `docker buildx bake ai_chat_lambda`
group "all" {
  targets = [
    "ai_chat_lambda",
    "audio_transcriber",
    "crud_api",
    "media_lambda",
    "render_job",
    "summarize_transcription",
    "twitch_lambda",
    "upload_video",
    "video_ingestor",
    "youtube_lambda",
  ]
}

target "ai_chat_lambda" {
  dockerfile = "Dockerfile"
  context = "."
  target = "ai_chat_lambda"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/ai-chat-lambda:latest"]
}

target "audio_transcriber" {
  dockerfile = "Dockerfile"
  context = "."
  target = "audio_transcriber"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/audio-transcriber:latest"]
}

target "crud_api" {
  dockerfile = "Dockerfile"
  context = "."
  target = "crud_api"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/crud-lambda:latest"]
}

target "media_lambda" {
  dockerfile = "Dockerfile"
  context = "."
  target = "media_lambda"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/media-lambda:latest"]
}

target "render_job" {
  dockerfile = "Dockerfile"
  context = "."
  target = "render_job"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/render-job:latest"]
}

target "summarize_transcription" {
  dockerfile = "Dockerfile"
  context = "."
  target = "summarize_transcription"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/summarize-transcription-lambda:latest"]
}

target "twitch_lambda" {
  dockerfile = "Dockerfile"
  context = "."
  target = "twitch_lambda"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/twitch-lambda:latest"]
}

target "upload_video" {
  dockerfile = "Dockerfile"
  context = "."
  target = "upload_video"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/upload-video:latest"]
}

target "video_ingestor" {
  dockerfile = "Dockerfile"
  context = "."
  target = "video_ingestor"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/video-ingestor:latest"]
}

target "youtube_lambda" {
  dockerfile = "Dockerfile"
  context = "."
  target = "youtube_lambda"
  tags = ["159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/youtube-lambda:latest"]
}