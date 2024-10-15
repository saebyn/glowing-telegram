"""
This module defines a Pulumi component resource that represents a video ingestor job and its associated resources, including an ECR repository and a Batch job definition.
"""

import pulumi
import pulumi_aws as aws
import pulumi_aws_native as aws_native
import json


class VideoIngestorJob(pulumi.ComponentResource):
    def __init__(
        self,
        name: str,
        video_archive: aws.s3.Bucket,
        output_bucket: aws.s3.Bucket,
        metadata_table: aws.dynamodb.Table,
        opts=None,
    ):
        super().__init__(
            "glowing_telegram:infrastructure:VideoIngestorJob", name, None, opts
        )

        # Create a repository for the video ingestor
        video_ingestor_repository = aws_native.ecr.Repository(
            f"{name}-repository",
            repository_name="video_ingestor",
            image_scanning_configuration={
                "scan_on_push": True,
            },
            opts=pulumi.ResourceOptions(
                parent=self,
                aliases=[pulumi.Alias("ecr-repository", parent=None)],
            ),
        )

        video_ingestor_image_tag = video_ingestor_repository.repository_uri.apply(
            lambda url: f"{url}:latest"
        )

        ## Create container execution role
        container_execution_role = aws_native.iam.Role(
            f"{name}-container-execution-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "ecs-tasks.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            managed_policy_arns=[
                "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy",
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        ## Create a task role for the video ingestor
        task_role = aws_native.iam.Role(
            f"{name}-task-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "ecs-tasks.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            policies=[
                aws_native.iam.RolePolicyArgs(
                    policy_name=f"{name}-task-policy",
                    policy_document={
                        "Version": "2012-10-17",
                        "Statement": [
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "s3:GetObject",
                                ],
                                "Resource": [
                                    pulumi.Output.format("{0}/*", video_archive.arn),
                                ],
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "s3:PutObject",
                                ],
                                "Resource": [
                                    pulumi.Output.format("{0}/*", output_bucket.arn),
                                ],
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "dynamodb:PutItem",
                                    "dynamodb:GetItem",
                                ],
                                "Resource": [
                                    pulumi.Output.format("{0}", metadata_table.arn),
                                ],
                            },
                        ],
                    },
                ),
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        ## Define the container properties for the video ingestor
        # Environment variables are passed to the container:
        # - INPUT_BUCKET: The name of the S3 bucket where videos are stored
        # - OUTPUT_BUCKET: The name of the S3 bucket where the results are stored
        # - KEYFRAMES_PREFIX: The prefix for the keyframes in the output bucket
        # - AUDIO_PREFIX: The prefix for the audio files in the output bucket
        # - DYNAMODB_TABLE: The name of the DynamoDB table where metadata is stored
        container_properties = pulumi.Output.all(
            video_archive.bucket_name,
            output_bucket.bucket_name,
            metadata_table.name,
            video_ingestor_image_tag,
            container_execution_role.arn,
            task_role.arn,
        ).apply(
            lambda args: json.dumps(
                dict(
                    command=["Ref::key"],
                    executionRoleArn=args[4],
                    jobRoleArn=args[5],
                    networkConfiguration={
                        "assignPublicIp": "ENABLED",
                    },
                    environment=[
                        {
                            "name": "INPUT_BUCKET",
                            "value": args[0],
                        },
                        {
                            "name": "OUTPUT_BUCKET",
                            "value": args[1],
                        },
                        {
                            "name": "KEYFRAMES_PREFIX",
                            "value": "keyframes",
                        },
                        {
                            "name": "AUDIO_PREFIX",
                            "value": "audio",
                        },
                        {
                            "name": "DYNAMODB_TABLE",
                            "value": args[2],
                        },
                        {"name": "SPEECH_TRACK_NUMBER", "value": "2"},
                        {
                            "name": "NOISE_TOLERANCE",
                            "value": "0.004",
                        },
                        {"name": "SILENCE_DURATION", "value": "30"},
                    ],
                    image=args[3],
                    resourceRequirements=[
                        {
                            "type": "VCPU",
                            "value": ".5",
                        },
                        {
                            "type": "MEMORY",
                            "value": "1024",
                        },
                    ],
                )
            )
        )

        ## Create a job definition for the video ingestor
        video_ingestor_job_definition = aws.batch.JobDefinition(
            f"{name}-job-definition",
            container_properties=container_properties,
            name="video-ingestor",
            parameters={
                "key": "<key>",
            },
            retry_strategy={
                "attempts": 1,
            },
            type="container",
            platform_capabilities=["FARGATE"],
            opts=pulumi.ResourceOptions(
                depends_on=[video_ingestor_repository],
                parent=self,
                aliases=[pulumi.Alias("video-ingestor-job-definition", parent=None)],
            ),
        )

        self.job_definition_arn = video_ingestor_job_definition.arn

        self.register_outputs(
            {
                "job_definition": video_ingestor_job_definition,
            }
        )
