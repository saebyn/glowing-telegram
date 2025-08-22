# Test Fixtures

This directory contains audio fixtures for integration testing.

## test_speech.wav

This file should contain English speech with the following transcript:

```
Hello, this is a test recording for the audio transcriber. I am speaking clearly and slowly. The quick brown fox jumps over the lazy dog. Testing, one, two, three.
```

**Recording Guidelines:**
- Format: WAV, 16-bit, 44.1kHz or 48kHz, mono or stereo
- Duration: Approximately 10-15 seconds
- Language: English
- Speech: Clear and moderately paced
- Environment: Quiet room with minimal background noise

The test expects the transcription to contain the key phrases:
- "test recording"
- "audio transcriber" 
- "quick brown fox"
- "lazy dog"
- "testing"

Note: The exact transcription may vary slightly due to Whisper's processing, but the key phrases should be present.
