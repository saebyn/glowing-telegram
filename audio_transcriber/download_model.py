from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor
import os

# Set cache directory for Hugging Face models
cache_dir = os.environ.get("HF_HOME", "/model")

# Download the whisper-large-v3 model
model_id = "openai/whisper-large-v3"

print(f"Downloading model: {model_id}")
print(f"Cache directory: {cache_dir}")

model = AutoModelForSpeechSeq2Seq.from_pretrained(
    model_id,
    cache_dir=cache_dir,
    use_safetensors=True
)

processor = AutoProcessor.from_pretrained(
    model_id,
    cache_dir=cache_dir
)

print("Model download complete!")
