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

The original `openai-whisper` package supported an `initial_prompt` parameter for conditioning the model. The transformers pipeline uses a different conditioning mechanism, so this parameter is currently accepted but not fully utilized in the same way.

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

## Future Improvements

Potential enhancements:
1. Better support for initial_prompt through prompt engineering
2. Audio chunking for clip_timestamps instead of processing full file
3. Support for streaming transcription
4. Fine-tuned models for specific use cases
