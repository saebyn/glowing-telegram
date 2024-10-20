"""glowing-telegram: A tool for managing stream recordings"""

import json
import pulumi
import pulumi_aws_native as aws_native
import pulumi_aws as aws

from FargateBatchJobQueue import FargateBatchJobQueue
from GPUBatchJobQueue import GPUBatchJobQueue
from VideoIngestorJob import VideoIngestorJob
from AudioTranscriberJob import AudioTranscriberJob
from StreamIngestion import StreamIngestion

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

# AWS Batch setup
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

fargate_batch_job_queue = FargateBatchJobQueue(
    "fargate-batch-job-queue",
    vpc_id=default_vpc.id,
    subnet_ids=default_subnets.ids,
)

gpu_batch_job_queue = GPUBatchJobQueue(
    "gpu-batch-job-queue",
    vpc_id=default_vpc.id,
    subnet_ids=default_subnets.ids,
)


video_ingestor_job = VideoIngestorJob(
    "video-ingestor-job",
    video_archive=video_archive,
    output_bucket=output_bucket,
    metadata_table=metadata_table,
)

audio_transcriber_job = AudioTranscriberJob(
    "audio-transcriber-job",
    output_bucket=output_bucket,
    metadata_table=metadata_table,
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
                            video_ingestor_job.job_definition_arn,
                            fargate_batch_job_queue.job_queue_arn,
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
            "arn": fargate_batch_job_queue.job_queue_arn,
            "role_arn": event_target_role.arn,
            "dead_letter_config": {
                "arn": dead_letter_queue.arn,
            },
            "batch_parameters": {
                "job_definition": video_ingestor_job.job_definition_arn,
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

StreamIngestion(
    "stream-ingestion",
    audio_transcriber_job_arn=audio_transcriber_job.job_definition_arn,
    gpu_batch_job_queue_arn=gpu_batch_job_queue.job_queue_arn,
    metadata_table=metadata_table,
)
