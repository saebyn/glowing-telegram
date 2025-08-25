# Embedding Service

This service generates embeddings for stream content and stores them in Aurora PostgreSQL with pgvector extension.

## Overview

The embedding service processes video clip transcriptions and summaries to create vector embeddings using OpenAI's embedding API. These embeddings are stored in an Aurora Serverless v2 PostgreSQL cluster with the pgvector extension for later retrieval and similarity search operations.

## Features

- **Independent operation**: Can be run standalone to process existing data
- **Multiple modes**: Support for processing all data, specific clips, or streams
- **Incremental processing**: Avoids re-processing existing embeddings
- **Multiple content types**: Generates embeddings for transcriptions and summaries
- **Vector similarity search**: Uses pgvector extension for efficient similarity queries
- **Automatic schema management**: Creates necessary tables and indexes on first run

## Usage

### Environment Variables

- `DYNAMODB_TABLE`: DynamoDB table containing video metadata
- `DATABASE_SECRET_ARN`: AWS Secrets Manager ARN for database credentials
- `DATABASE_ENDPOINT`: Hostname of the Aurora PostgreSQL cluster
- `DATABASE_PORT`: Port number for database connection (defaults to 5432)
- `DATABASE_NAME`: Name of the database to connect to (typically 'vectors')
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

## Database Schema

The service automatically creates the required database schema:

```sql
CREATE TABLE embeddings (
    id TEXT PRIMARY KEY,
    stream_id TEXT NOT NULL,
    video_key TEXT NOT NULL,
    content TEXT NOT NULL,
    content_type TEXT NOT NULL,
    embedding vector(1536),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);
```

Includes indexes for efficient querying:
- `idx_embeddings_stream_id` - for filtering by stream
- `idx_embeddings_video_key` - for filtering by video  
- `idx_embeddings_content_type` - for filtering by content type
- `idx_embeddings_embedding_hnsw` - HNSW index for fast similarity search