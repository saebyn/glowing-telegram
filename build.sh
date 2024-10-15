#!/bin/bash
docker build -f audio_transcriber/Dockerfile -t audio_transcriber .
docker tag audio_transcriber 159222827421.dkr.ecr.us-west-2.amazonaws.com/audio_transcriber:latest
docker push 159222827421.dkr.ecr.us-west-2.amazonaws.com/audio_transcriber:latest

docker build -f video_ingestor/Dockerfile -t video_ingestor .
docker tag video_ingestor 159222827421.dkr.ecr.us-west-2.amazonaws.com/video_ingestor:latest
docker push 159222827421.dkr.ecr.us-west-2.amazonaws.com/video_ingestor:latest