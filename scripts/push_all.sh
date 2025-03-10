#!/bin/sh

export AWS_PROFILE=glowing-telegram-admin
export AWS_REGION=us-west-2

# Generate the ECR URI
ECR_DOMAIN=159222827421.dkr.ecr.us-west-2.amazonaws.com

# Login to ECR
aws ecr get-login-password | docker login --username AWS --password-stdin $ECR_DOMAIN

docker buildx bake all --push