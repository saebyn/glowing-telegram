#!/bin/sh


# Generate the ECR URI
ECR_DOMAIN=$AWS_ACCOUNT_ID.dkr.ecr.$AWS_REGION.amazonaws.com

# Login to ECR
aws ecr get-login-password | docker login --username AWS --password-stdin $ECR_DOMAIN

docker buildx bake all --push