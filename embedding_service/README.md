# Embedding Service

This service generates embeddings for stream content and stores them in an S3 vector bucket.

## Overview

The embedding service processes video clip transcriptions and summaries to create vector embeddings using OpenAI's embedding API. These embeddings are stored in S3 for later retrieval and similarity search.

## Features

- **Independent operation**: Can be run standalone to process existing data
- **Multiple modes**: Support for processing all data, specific clips, or streams
- **Incremental processing**: Avoids re-processing existing embeddings
- **Multiple content types**: Generates embeddings for transcriptions and summaries
- **Structured storage**: Stores embeddings with metadata in JSON format

## Usage

### Environment Variables

- `DYNAMODB_TABLE`: DynamoDB table containing video metadata
- `VECTOR_BUCKET`: S3 bucket for storing embeddings  
- `OPENAI_SECRET_ARN`: AWS Secrets Manager ARN for OpenAI API key
- `OPENAI_MODEL`: OpenAI model to use (default: text-embedding-3-small)

### Commands

```bash
# Scan all existing data for embedding generation
./embedding_service scan

# Process a specific video clip
./embedding_service process <video_key>

# Process all clips for a specific stream
./embedding_service scan-stream <stream_id>
```

## Integration

The service is integrated into the stream ingestion step function and runs automatically after transcription summarization is complete. It can also be run independently to process historical data.

## Output Format

Embeddings are stored as JSON files in S3 with the following structure:

```json
[
  {
    "id": "video_key:content_type",
    "stream_id": "stream_uuid",
    "video_key": "path/to/video.mp4",
    "content": "transcribed or summarized text",
    "content_type": "transcription|summary",
    "embedding": [0.1, 0.2, ...],
    "timestamp": "2024-01-01T00:00:00Z",
    "metadata": {}
  }
]
```