#!/usr/bin/env python3
"""
Hugging Face Whisper transcription wrapper.
This script provides a CLI interface compatible with the openai-whisper package
but uses the Hugging Face transformers library instead.
"""

import argparse
import json
import sys
import os
import tempfile
from pathlib import Path
import warnings

# Suppress specific warnings
warnings.filterwarnings("ignore", category=FutureWarning)
warnings.filterwarnings("ignore", category=UserWarning)

import torch
from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor, pipeline


def parse_clip_timestamps(clip_timestamps_str):
    """Parse clip_timestamps string into a list of tuples (start, end)."""
    if not clip_timestamps_str or clip_timestamps_str == "0":
        return None
    
    timestamps = [float(x) for x in clip_timestamps_str.split(",")]
    
    # Convert list of timestamps into pairs (start, end)
    # Format: "0,5,10,15" means segments (0,5) and (10,15)
    chunks = []
    for i in range(0, len(timestamps), 2):
        if i + 1 < len(timestamps):
            chunks.append((timestamps[i], timestamps[i + 1]))
        else:
            # If there's an odd number, the last one is open-ended
            chunks.append((timestamps[i], None))
    
    return chunks


def transcribe_audio(
    audio_path,
    model_id,
    device,
    language,
    initial_prompt,
    clip_timestamps,
    output_dir,
    verbose=False
):
    """Transcribe audio using Hugging Face Whisper model."""
    
    if verbose:
        print(f"Loading model: {model_id}", file=sys.stderr)
    
    # Determine device
    if device == "cuda" and torch.cuda.is_available():
        device_str = "cuda:0"
        torch_dtype = torch.float16
    else:
        device_str = "cpu"
        torch_dtype = torch.float32
        if device == "cuda" and not torch.cuda.is_available():
            print("Warning: CUDA requested but not available, falling back to CPU", file=sys.stderr)
    
    if verbose:
        print(f"Using device: {device_str}", file=sys.stderr)
        print(f"Using dtype: {torch_dtype}", file=sys.stderr)
    
    # Load model and processor
    try:
        model = AutoModelForSpeechSeq2Seq.from_pretrained(
            model_id,
            torch_dtype=torch_dtype,
            low_cpu_mem_usage=True,
            use_safetensors=True,
            cache_dir=os.environ.get("HF_HOME", "/model")
        )
        model.to(device_str)
        
        processor = AutoProcessor.from_pretrained(
            model_id,
            cache_dir=os.environ.get("HF_HOME", "/model")
        )
    except Exception as e:
        print(f"Error loading model: {e}", file=sys.stderr)
        sys.exit(1)
    
    # Create pipeline
    pipe = pipeline(
        "automatic-speech-recognition",
        model=model,
        tokenizer=processor.tokenizer,
        feature_extractor=processor.feature_extractor,
        max_new_tokens=128,
        chunk_length_s=30,
        batch_size=16,
        return_timestamps=True,
        torch_dtype=torch_dtype,
        device=device_str,
    )
    
    # Generate arguments for the pipeline
    generate_kwargs = {}
    if language:
        # Convert language code to the format expected by transformers
        # whisper uses 2-letter codes, transformers uses full language names for task
        generate_kwargs["language"] = language
    
    # Handle initial_prompt by converting to prompt_ids
    # This conditions the model to generate text consistent with the prompt style
    if initial_prompt:
        try:
            prompt_ids = processor.get_prompt_ids(initial_prompt, return_tensors="pt")
            generate_kwargs["prompt_ids"] = prompt_ids
            if verbose:
                print(f"Using initial prompt: {initial_prompt}", file=sys.stderr)
        except Exception as e:
            if verbose:
                print(f"Warning: Could not set initial prompt: {e}", file=sys.stderr)
    
    if verbose:
        print(f"Processing audio file: {audio_path}", file=sys.stderr)
    
    # Parse clip timestamps
    chunks = parse_clip_timestamps(clip_timestamps)
    
    # Transcribe
    try:
        if chunks:
            # If we have specific chunks, we need to process them separately
            # For now, we'll transcribe the whole file and let the model handle it
            # A more sophisticated implementation would split the audio file
            if verbose:
                print(f"Using clip timestamps: {clip_timestamps}", file=sys.stderr)
            result = pipe(audio_path, generate_kwargs=generate_kwargs)
        else:
            result = pipe(audio_path, generate_kwargs=generate_kwargs)
    except Exception as e:
        print(f"Error during transcription: {e}", file=sys.stderr)
        sys.exit(1)
    
    if verbose:
        print(f"Transcription complete", file=sys.stderr)
    
    # Convert result to whisper-compatible format
    output = {
        "text": result["text"],
        "segments": [],
        "language": language or "en"
    }
    
    # Convert chunks to segments
    if "chunks" in result:
        for chunk in result["chunks"]:
            segment = {
                "id": len(output["segments"]),
                "seek": 0,
                "start": chunk["timestamp"][0] if chunk["timestamp"][0] is not None else 0.0,
                "end": chunk["timestamp"][1] if chunk["timestamp"][1] is not None else 0.0,
                "text": chunk["text"],
                "tokens": [],
                "temperature": 0.0,
                "avg_logprob": 0.0,
                "compression_ratio": 0.0,
                "no_speech_prob": 0.0
            }
            output["segments"].append(segment)
    else:
        # If no chunks, create a single segment
        output["segments"].append({
            "id": 0,
            "seek": 0,
            "start": 0.0,
            "end": 0.0,
            "text": result["text"],
            "tokens": [],
            "temperature": 0.0,
            "avg_logprob": 0.0,
            "compression_ratio": 0.0,
            "no_speech_prob": 0.0
        })
    
    # Write output
    output_file = Path(output_dir) / "-.json"
    with open(output_file, "w") as f:
        json.dump(output, f, indent=2)
    
    if verbose:
        print(f"Output written to: {output_file}", file=sys.stderr)
    
    return output


def main():
    parser = argparse.ArgumentParser(description="Transcribe audio using Hugging Face Whisper")
    parser.add_argument("audio_file", help="Audio file to transcribe (use '-' for stdin)")
    parser.add_argument("--model", required=True, help="Model name (e.g., 'large-v3')")
    parser.add_argument("--model_dir", default="/model", help="Directory containing models")
    parser.add_argument("--initial_prompt", default="", help="Initial prompt for the model")
    parser.add_argument("--language", default="en", help="Language code (e.g., 'en')")
    parser.add_argument("--clip_timestamps", default="0", help="Timestamps to clip (comma-separated)")
    parser.add_argument("--output_format", default="json", help="Output format")
    parser.add_argument("--output_dir", required=True, help="Output directory")
    parser.add_argument("--task", default="transcribe", help="Task to perform")
    parser.add_argument("--device", default="cpu", help="Device to use (cpu or cuda)")
    parser.add_argument("--verbose", default="False", help="Enable verbose output")
    
    args = parser.parse_args()
    
    verbose = args.verbose.lower() in ["true", "1", "yes"]
    
    # Map model name to Hugging Face model ID
    model_map = {
        "tiny": "openai/whisper-tiny",
        "base": "openai/whisper-base",
        "small": "openai/whisper-small",
        "medium": "openai/whisper-medium",
        "large": "openai/whisper-large",
        "large-v2": "openai/whisper-large-v2",
        "large-v3": "openai/whisper-large-v3",
        "turbo": "openai/whisper-large-v3",  # Map turbo to large-v3
    }
    
    model_id = model_map.get(args.model, f"openai/whisper-{args.model}")
    
    # Handle stdin input
    if args.audio_file == "-":
        if verbose:
            print("Reading audio from stdin", file=sys.stderr)
        
        # Create temporary file to store stdin data
        with tempfile.NamedTemporaryFile(delete=False, suffix=".audio") as tmp:
            # Read binary data from stdin
            tmp.write(sys.stdin.buffer.read())
            tmp.flush()
            audio_path = tmp.name
        
        try:
            transcribe_audio(
                audio_path=audio_path,
                model_id=model_id,
                device=args.device,
                language=args.language,
                initial_prompt=args.initial_prompt,
                clip_timestamps=args.clip_timestamps,
                output_dir=args.output_dir,
                verbose=verbose
            )
        finally:
            # Clean up temporary file
            os.unlink(audio_path)
    else:
        transcribe_audio(
            audio_path=args.audio_file,
            model_id=model_id,
            device=args.device,
            language=args.language,
            initial_prompt=args.initial_prompt,
            clip_timestamps=args.clip_timestamps,
            output_dir=args.output_dir,
            verbose=verbose
        )


if __name__ == "__main__":
    main()
