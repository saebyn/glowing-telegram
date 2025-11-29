# Migration from openai-whisper to Hugging Face Transformers

## Overview

This document describes the migration from the `openai-whisper` pip package to the Hugging Face `transformers` library for audio transcription.

## Motivation

The original `openai-whisper` package had issues with long silences breaking or hanging transcription. The Hugging Face transformers implementation provides better handling of these edge cases.

## Changes Made

### 1. Python Dependencies

**Before:**
```dockerfile
RUN pip3 install --break-system-packages openai-whisper
```

**After:**
```dockerfile
RUN pip3 install --break-system-packages transformers torch torchaudio accelerate
```

### 2. Model Download

**Before (`download_model.py`):**
```python
import whisper
whisper.load_model("turbo", download_root="/model")
```

**After (`download_model.py`):**
```python
from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor
model = AutoModelForSpeechSeq2Seq.from_pretrained(
    "openai/whisper-large-v3",
    cache_dir="/model",
    use_safetensors=True
)
processor = AutoProcessor.from_pretrained(
    "openai/whisper-large-v3",
    cache_dir="/model"
)
```

### 3. Transcription Interface

A new wrapper script `whisper_hf.py` was created to maintain CLI compatibility.

**Before (Rust code):**
```rust
Command::new("whisper")
    .args([
        "--model", model_str,
        "--language", &language,
        // ... other args
        "-"
    ])
```

**After (Rust code):**
```rust
Command::new("python3")
    .args([
        "/app/whisper_hf.py",
        "-",
        "--model", model_str,
        "--language", &language,
        // ... other args (same as before)
    ])
```

## Model Mapping

The wrapper script maps model names to Hugging Face model IDs:

| Original Model | Hugging Face Model ID |
|----------------|----------------------|
| tiny           | openai/whisper-tiny |
| base           | openai/whisper-base |
| small          | openai/whisper-small |
| medium         | openai/whisper-medium |
| large          | openai/whisper-large |
| large-v2       | openai/whisper-large-v2 |
| large-v3       | openai/whisper-large-v3 |
| turbo          | openai/whisper-large-v3 |

## Behavioral Differences

### Initial Prompt

The `initial_prompt` parameter is now supported using the transformers `prompt_ids` feature. This converts the prompt text into token IDs that condition the model's generation, helping it maintain consistent style, terminology, and formatting.

Example usage:
```bash
./audio_transcriber key audio.wav "Gaming stream with viewer interactions" "en"
```

The prompt helps the model understand context like:
- Technical terminology to expect
- Speaker style and tone
- Domain-specific vocabulary

### Clip Timestamps

The `--clip_timestamps` feature is preserved and works the same way as before, parsing comma-separated timestamp pairs to process specific audio segments.

### Device Handling

The new implementation provides better fallback handling:
- If CUDA is requested but not available, automatically falls back to CPU with a warning
- Explicit device selection: `--device cpu` or `--device cuda`

## Output Format

The output JSON format remains compatible with the original whisper format:

```json
{
  "text": "Full transcription text",
  "segments": [
    {
      "id": 0,
      "seek": 0,
      "start": 0.0,
      "end": 5.0,
      "text": "Segment text",
      "tokens": [],
      "temperature": 0.0,
      "avg_logprob": 0.0,
      "compression_ratio": 0.0,
      "no_speech_prob": 0.0
    }
  ],
  "language": "en"
}
```

## Testing

### Manual Testing

To test the new implementation locally:

```bash
# Build the Docker container
docker buildx bake audio_transcriber

# Run with LocalStack or real AWS resources
docker run --rm \
  -e INPUT_BUCKET=test-bucket \
  -e DYNAMODB_TABLE=test-table \
  -e AWS_ENDPOINT_URL=http://localhost:4566 \
  audio_transcriber \
  test-key \
  test-audio.wav \
  "initial prompt" \
  "en"
```

### Integration Tests

The existing integration tests in `audio_transcriber/tests/` should continue to work:

```bash
cd audio_transcriber
cargo test --test integration_test
```

## Troubleshooting

### Model Download Issues

If model download fails, check:
- Network connectivity to huggingface.co
- Disk space in the `/model` directory
- HF_HOME environment variable is set correctly

### CUDA Issues

If you see CUDA errors:
- Verify CUDA is properly installed
- Check GPU is accessible in the container
- Try forcing CPU mode with `DEVICE=cpu` environment variable

### Transcription Quality

If transcription quality differs:
- The whisper-large-v3 model may produce different results than turbo
- Try adjusting the language parameter
- Check that audio quality is sufficient

## Rollback Plan

To rollback to the original implementation:

1. Restore `Dockerfile`:
   ```dockerfile
   RUN pip3 install --break-system-packages openai-whisper
   ```

2. Restore `audio_transcriber/src/whisper.rs`:
   ```rust
   Command::new("whisper")
   ```

3. Restore `download_model.py`:
   ```python
   import whisper
   whisper.load_model("turbo", download_root="/model")
   ```

4. Remove `audio_transcriber/whisper_hf.py`

## Performance Considerations

- **Model Size**: whisper-large-v3 is larger than turbo, requiring more disk space and memory
- **Inference Speed**: May be slightly slower or faster depending on hardware and implementation
- **GPU Memory**: Large-v3 requires more GPU memory than smaller models

## AWS Batch Deployment with EFS Model Cache

The CDK infrastructure provisions an EFS file system for caching HuggingFace models, eliminating the need to bake the ~3GB model into the Docker image.

### How EFS Model Caching Works

1. **EFS File System**: A dedicated EFS file system (`ModelCacheFileSystem`) is created in the VPC with:
   - General Purpose performance mode for balanced latency/throughput
   - Bursting throughput mode to handle model download spikes
   - 30-day lifecycle policy to move infrequently accessed files to cheaper storage
   - RETAIN removal policy to preserve models across stack updates

2. **Access Point**: An EFS access point is configured with:
   - Path: `/models` - all model files are stored here
   - POSIX user: UID/GID 10001 (matches container user)
   - Permissions: 755 (owner read/write/execute, others read/execute)

3. **Volume Mount**: The Batch job mounts EFS at `/mnt/efs/models` with:
   - Transit encryption enabled for security
   - Job role authentication for access control

4. **Environment Variable**: `HF_HOME=/mnt/efs/models` tells HuggingFace where to cache models

### First Run Behavior

On the first job execution (or when the model isn't cached):

1. Container starts and checks `HF_HOME` for cached model
2. Model not found → downloads from HuggingFace Hub (~3GB)
3. Model saved to EFS at `/mnt/efs/models/models--openai--whisper-large-v3/`
4. Transcription proceeds normally

**First run takes ~5-10 minutes** depending on network speed. Subsequent runs start immediately.

### Cached Model Structure

```
/mnt/efs/models/
├── models--openai--whisper-large-v3/
│   ├── blobs/           # Model weights (safetensors)
│   ├── refs/            # Version references
│   └── snapshots/       # Versioned model files
└── hub/                 # Additional HuggingFace cache files
```

### EFS Troubleshooting

#### Job Fails with "Mount Target Not Found"

**Symptoms**: Job fails immediately with EFS mount errors

**Cause**: EFS mount targets not available in the subnet

**Solution**:
1. Check EFS mount targets exist in the VPC subnets:
   ```bash
   aws efs describe-mount-targets --file-system-id fs-xxxxx
   ```
2. Verify security group allows NFS (port 2049) from Batch compute instances
3. Ensure subnets have route to EFS endpoints

#### Job Hangs During Model Download

**Symptoms**: Job runs for extended time without progress

**Cause**: Network connectivity issues to HuggingFace Hub

**Solution**:
1. Check VPC has internet access (NAT Gateway or public subnet)
2. Verify no firewall blocking `huggingface.co` and `cdn-lfs.huggingface.co`
3. Check CloudWatch logs for download progress:
   ```bash
   aws logs get-log-events --log-group-name /aws/batch/job --log-stream-name <job-id>
   ```

#### Permission Denied Errors

**Symptoms**: "Permission denied" when accessing `/mnt/efs/models`

**Cause**: POSIX user mismatch between container and EFS access point

**Solution**:
1. Verify container runs as UID 10001:
   ```dockerfile
   USER 10001:10001
   ```
2. Check EFS access point configuration:
   ```bash
   aws efs describe-access-points --file-system-id fs-xxxxx
   ```
3. Ensure access point has correct `PosixUser` (uid: 10001, gid: 10001)

#### Model Corrupted or Incomplete

**Symptoms**: Model loading fails with checksum or parsing errors

**Cause**: Previous download was interrupted

**Solution**: Reset the model cache (see below)

### Resetting/Redownloading the Model

#### Option 1: Delete Specific Model (Recommended)

Remove only the whisper model, preserving other cached models:

```bash
# Connect to EFS from an EC2 instance or use AWS DataSync
# Mount EFS
sudo mount -t efs fs-xxxxx:/ /mnt/efs

# Remove whisper model cache
sudo rm -rf /mnt/efs/models/models--openai--whisper-large-v3/

# Unmount
sudo umount /mnt/efs
```

Next job will re-download the model.

#### Option 2: Clear All Model Cache

Remove all cached models:

```bash
sudo mount -t efs fs-xxxxx:/ /mnt/efs
sudo rm -rf /mnt/efs/models/*
sudo umount /mnt/efs
```

#### Option 3: Using AWS CLI with EFS Access Point

If you have an EC2 instance in the same VPC:

```bash
# Install EFS mount helper
sudo yum install -y amazon-efs-utils

# Mount using access point
sudo mount -t efs -o tls,accesspoint=fsap-xxxxx fs-xxxxx:/ /mnt/efs

# Clear cache
sudo rm -rf /mnt/efs/*

sudo umount /mnt/efs
```

#### Option 4: Force Re-download via Environment Variable

Set `HF_HUB_OFFLINE=0` and `TRANSFORMERS_CACHE=/tmp/fresh` to bypass EFS cache for a single job (useful for testing):

```json
{
  "containerOverrides": {
    "environment": [
      {"name": "HF_HUB_OFFLINE", "value": "0"},
      {"name": "HF_HOME", "value": "/tmp/fresh-download"}
    ]
  }
}
```

Note: This downloads to the container's ephemeral storage, not EFS.

### Monitoring EFS Usage

#### Check EFS Storage Size

```bash
aws efs describe-file-systems --file-system-id fs-xxxxx \
  --query 'FileSystems[0].SizeInBytes'
```

Expected size after model download: ~3-4 GB

#### CloudWatch Metrics

Monitor these EFS metrics in CloudWatch:
- `BurstCreditBalance` - Ensure burst credits don't deplete
- `ClientConnections` - Active NFS connections from Batch jobs
- `DataReadIOBytes` / `DataWriteIOBytes` - I/O activity

### Alternative Deployment Options

#### Option 1: Bake Model into Image

For simpler deployments or when cold-start time is critical:

```bash
# Default build includes model (~5GB image)
docker buildx bake audio_transcriber
```

#### Option 2: S3 Model Storage

Use Hugging Face Hub's S3 support:

```python
from huggingface_hub import snapshot_download
snapshot_download("openai/whisper-large-v3", cache_dir="/mnt/efs/models")
```

### Comparison

| Approach | Image Size | Cold Start | Model Updates | Infrastructure |
|----------|------------|------------|---------------|----------------|
| EFS Cache | ~2GB | First run slow | Automatic | EFS file system |
| Baked Image | ~5GB | Fast | Rebuild required | None |
| S3 Storage | ~2GB | Medium | Manual sync | S3 bucket |

## Future Improvements

Potential enhancements:
1. Better support for initial_prompt through prompt engineering
2. Audio chunking for clip_timestamps instead of processing full file
3. Support for streaming transcription
4. Fine-tuned models for specific use cases
