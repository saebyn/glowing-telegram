"""
This module defines a Pulumi component resource that represents an audio transcriber job and its associated resources, including an ECR repository and a Batch job definition.
"""

import pulumi
import pulumi_aws as aws
import pulumi_aws_native as aws_native
import json


class AudioTranscriberJob(pulumi.ComponentResource):
    def __init__(
        self,
        name: str,
        output_bucket: aws.s3.Bucket,
        metadata_table: aws.dynamodb.Table,
        opts=None,
    ):
        super().__init__(
            "glowing_telegram:infrastructure:AudioTranscriberJob", name, None, opts
        )

        repository = aws_native.ecr.Repository(
            f"{name}-repository",
            repository_name="audio_transcriber",
            image_scanning_configuration={
                "scan_on_push": True,
            },
            opts=pulumi.ResourceOptions(
                parent=self,
            ),
        )

        audio_transcriber_image_tag = repository.repository_uri.apply(
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
                                    pulumi.Output.format("{0}/*", output_bucket.arn),
                                ],
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "dynamodb:UpdateItem",
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

        ## Define the container properties for the audio transcriber job
        # Environment variables are passed to the container:
        # - INPUT_BUCKET: The name of the S3 bucket where audio files are stored
        # - DYNAMODB_TABLE: The name of the DynamoDB table where metadata is stored
        container_properties = pulumi.Output.all(
            output_bucket.bucket_name,
            metadata_table.name,
            audio_transcriber_image_tag,
            container_execution_role.arn,
            task_role.arn,
        ).apply(
            lambda args: json.dumps(
                dict(
                    command=[
                        "Ref::item_key",
                        "Ref::input_key",
                        "Ref::initial_prompt",
                        "Ref::language",
                    ],
                    executionRoleArn=args[3],
                    jobRoleArn=args[4],
                    environment=[
                        {
                            "name": "INPUT_BUCKET",
                            "value": args[0],
                        },
                        {
                            "name": "DYNAMODB_TABLE",
                            "value": args[1],
                        },
                        {
                            "name": "NVIDIA_DRIVER_CAPABILITIES",
                            "value": "all",
                        },
                        {
                            "name": "RUST_LOG",
                            "value": "info",
                        },
                    ],
                    image=args[2],
                    resourceRequirements=[
                        {"type": "VCPU", "value": "1"},
                        {"type": "MEMORY", "value": "8192"},
                        {"type": "GPU", "value": "1"},
                    ],
                )
            )
        )

        job_definition = aws.batch.JobDefinition(
            f"{name}-job-definition",
            container_properties=container_properties,
            name="audio-transcriber",
            timeout={
                "attempt_duration_seconds": 5 * 60,  # 5 minutes
            },
            parameters={
                "item_key": "<item_key>",
                "input_key": "<input_key>",
                "initial_prompt": "<initial_prompt>",
                "language": "<language>",
            },
            retry_strategy={
                "attempts": 1,
            },
            type="container",
            platform_capabilities=["EC2"],
            opts=pulumi.ResourceOptions(
                depends_on=[repository],
                parent=self,
            ),
        )

        self.job_definition_arn = job_definition.arn

        self.register_outputs(
            {
                "job_definition": job_definition,
            }
        )
