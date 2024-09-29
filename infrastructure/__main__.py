"""glowing-telegram: A tool for managing stream recordings"""

import json
import pulumi
import pulumi_aws_native as aws_native
import pulumi_aws as aws

video_archive = aws_native.s3.Bucket(
    "video-archive",
    accelerate_configuration={
        "acceleration_status": aws_native.s3.BucketAccelerateConfigurationAccelerationStatus.ENABLED,
    },
    analytics_configurations=[
        {
            "id": "test",
            "storage_class_analysis": {},
        }
    ],
    bucket_encryption={
        "server_side_encryption_configuration": [
            {
                "bucket_key_enabled": True,
                "server_side_encryption_by_default": {
                    "sse_algorithm": aws_native.s3.BucketServerSideEncryptionByDefaultSseAlgorithm.AES256,
                },
            }
        ],
    },
    bucket_name="saebyn-video-archive",
    lifecycle_configuration={
        "rules": [
            {
                "abort_incomplete_multipart_upload": {
                    "days_after_initiation": 1,
                },
                "id": "delete_old_markers",
                "status": aws_native.s3.BucketRuleStatus.ENABLED,
            },
            {
                "id": "glacier archive",
                "status": aws_native.s3.BucketRuleStatus.ENABLED,
                "transitions": [
                    {
                        "storage_class": aws_native.s3.BucketTransitionStorageClass.STANDARD_IA,
                        "transition_in_days": 30,
                    },
                    {
                        "storage_class": aws_native.s3.BucketTransitionStorageClass.GLACIER_IR,
                        "transition_in_days": 60,
                    },
                    {
                        "storage_class": aws_native.s3.BucketTransitionStorageClass.GLACIER,
                        "transition_in_days": 150,
                    },
                ],
            },
        ],
    },
    ownership_controls={
        "rules": [
            {
                "object_ownership": aws_native.s3.BucketOwnershipControlsRuleObjectOwnership.BUCKET_OWNER_ENFORCED,
            }
        ],
    },
    public_access_block_configuration={
        "block_public_acls": True,
        "block_public_policy": True,
        "ignore_public_acls": True,
        "restrict_public_buckets": True,
    },
    versioning_configuration={
        "status": aws_native.s3.BucketVersioningConfigurationStatus.ENABLED,
    },
    notification_configuration={
        "event_bridge_configuration": {
            "event_bridge_enabled": True,
        },
    },
    opts=pulumi.ResourceOptions(protect=True),
)

# Output bucket
output_bucket = aws_native.s3.Bucket(
    "output-bucket",
    opts=pulumi.ResourceOptions(protect=True),
)

# Create a DynamoDB table to store metadata about the videos
metadata_table = aws.dynamodb.Table(
    "metadata-table",
    billing_mode="PAY_PER_REQUEST",
    hash_key="key",
    attributes=[
        {
            "name": "key",
            "type": "S",
        },
    ],
    opts=pulumi.ResourceOptions(protect=True),
)

# Create a repository for the video ingestor
ecr_repository = aws_native.ecr.Repository(
    "ecr-repository",
    repository_name="video_ingestor",
    image_scanning_configuration={
        "scan_on_push": True,
    },
)

video_ingestor_image_tag = ecr_repository.repository_uri.apply(
    lambda url: f"{url}:latest"
)

# AWS Batch compute environment

## Get the default VPC
default_vpc = aws.ec2.get_vpc(default=True)

## Get subnets in the default VPC
default_subnets = aws.ec2.get_subnets_output(
    filters=[
        {
            "name": "vpc-id",
            "values": [default_vpc.id],
        },
    ]
)

## Create a service role for the compute environment
compute_environment_service_role = aws_native.iam.Role(
    "compute-environment-service-role",
    assume_role_policy_document={
        "Version": "2012-10-17",
        "Statement": [
            {
                "Effect": "Allow",
                "Principal": {
                    "Service": "batch.amazonaws.com",
                },
                "Action": "sts:AssumeRole",
            },
        ],
    },
    managed_policy_arns=[
        "arn:aws:iam::aws:policy/service-role/AWSBatchServiceRole",
    ],
)

## Create a security group for the compute environment
compute_environment_security_group = aws_native.ec2.SecurityGroup(
    "compute-environment-security-group",
    vpc_id=default_vpc.id,
    group_description="Security group for the compute environment",
    security_group_egress=[
        {
            "cidr_ip": "0.0.0.0/0",
            "from_port": 0,
            "ip_protocol": "-1",
            "to_port": 0,
        },
    ],
)

## Create a compute environment for AWS Batch
compute_environment = aws_native.batch.ComputeEnvironment(
    "compute-environment",
    compute_environment_name="video-ingestor-compute-environment",
    compute_resources={
        "maxv_cpus": 16,
        "security_group_ids": [compute_environment_security_group.id],
        "subnets": default_subnets.ids,
        "type": "FARGATE",
    },
    service_role=compute_environment_service_role.arn,
    type="MANAGED",
)

## Create an AWS batch queue
batch_queue = aws_native.batch.JobQueue(
    "batch-queue",
    compute_environment_order=[
        {
            "compute_environment": compute_environment.compute_environment_arn,
            "order": 1,
        },
    ],
    priority=1,
)

## Create container execution role
container_execution_role = aws_native.iam.Role(
    "container-execution-role",
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
)

## Create a task role for the video ingestor
task_role = aws_native.iam.Role(
    "video-ingestor-task-role",
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
            policy_name="video-ingestor-task-policy",
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
            command=["/app/runtime", "Ref::key"],
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
                {"name": "SPEECH_TRACK_NUMBER", "value": 1},
                {
                    "name": "NOISE_TOLERANCE",
                    "value": 0.004,
                },
                {"name": "SILENCE_DURATION", "value": 30},
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
                # {
                #     "type": "GPU",
                #     "value": "1",
                # },
            ],
        )
    )
)

## Create a job definition for the video ingestor
video_ingestor_job_definition = aws.batch.JobDefinition(
    "video-ingestor-job-definition",
    container_properties=container_properties,
    name="video-ingestor-job-definition",
    parameters={
        "key": "<key>",
    },
    retry_strategy={
        "attempts": 1,
    },
    type="container",
    opts=pulumi.ResourceOptions(depends_on=[ecr_repository]),
    platform_capabilities=["FARGATE"],
)

## Create a role for the target of the event rule
event_target_role = aws_native.iam.Role(
    "event-target-role",
    assume_role_policy_document={
        "Version": "2012-10-17",
        "Statement": [
            {
                "Effect": "Allow",
                "Principal": {
                    "Service": "events.amazonaws.com",
                },
                "Action": "sts:AssumeRole",
            },
        ],
    },
    policies=[
        aws_native.iam.RolePolicyArgs(
            policy_name="event-target-policy",
            policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Action": [
                            "batch:SubmitJob",
                        ],
                        "Resource": [
                            video_ingestor_job_definition.arn,
                            batch_queue.job_queue_arn,
                        ],
                    },
                ],
            },
        ),
    ],
)

## Create a dead letter queue for the event rule
dead_letter_queue = aws_native.sqs.Queue(
    "dead-letter-queue", message_retention_period=1209600
)


## Create a event rule to trigger our batch job
event_rule = aws_native.events.Rule(
    "event-rule",
    state=aws_native.events.RuleState.ENABLED,
    event_pattern={
        "source": ["aws.s3"],
        "detail-type": ["Object Created"],
        "detail": {
            "bucket": {"name": [video_archive.bucket_name]},
        },
    },
    targets=[
        {
            "id": "video-ingestor",
            "arn": batch_queue.job_queue_arn,
            "role_arn": event_target_role.arn,
            "dead_letter_config": {
                "arn": dead_letter_queue.arn,
            },
            "batch_parameters": {
                "job_definition": video_ingestor_job_definition.arn,
                "job_name": "video-ingestor",
            },
            "input_transformer": {
                "input_paths_map": {
                    "key": "$.detail.object.key",
                },
                "input_template": '{"Parameters": {"key": <key>}}',
            },
        },
    ],
)
