"""glowing-telegram: A tool for managing stream recordings"""

import pulumi
import pulumi_aws_native as aws_native
import pulumi_aws as aws

from FargateBatchJobQueue import FargateBatchJobQueue
from GPUBatchJobQueue import GPUBatchJobQueue
from VideoIngestorJob import VideoIngestorJob
from AudioTranscriberJob import AudioTranscriberJob
from StreamIngestion import StreamIngestion
from API import API

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

# Create a DynamoDB tables for the application
video_metadata_table = aws.dynamodb.Table(
    "metadata-table",
    billing_mode="PAY_PER_REQUEST",
    hash_key="key",
    attributes=[
        {
            "name": "key",
            "type": "S",
        },
        {
            "name": "stream_id",
            "type": "S",
        },
    ],
    global_secondary_indexes=[
        {
            "hash_key": "stream_id",
            "name": "stream_id-index",
            "projection_type": "ALL",
        }
    ],
    opts=pulumi.ResourceOptions(protect=True),
)

streams_table = aws.dynamodb.Table(
    "streams",
    billing_mode="PAY_PER_REQUEST",
    hash_key="id",
    attributes=[
        {
            "name": "id",
            "type": "S",
        }
    ],
    opts=pulumi.ResourceOptions(protect=True),
)

stream_series_table = aws.dynamodb.Table(
    "stream-series",
    billing_mode="PAY_PER_REQUEST",
    hash_key="id",
    attributes=[
        {
            "name": "id",
            "type": "S",
        }
    ],
    opts=pulumi.ResourceOptions(protect=True),
)

episodes_table = aws.dynamodb.Table(
    "episodes",
    billing_mode="PAY_PER_REQUEST",
    hash_key="id",
    attributes=[
        {
            "name": "id",
            "type": "S",
        }
    ],
    opts=pulumi.ResourceOptions(protect=True),
)

profiles_table = aws.dynamodb.Table(
    "profiles",
    billing_mode="PAY_PER_REQUEST",
    hash_key="id",
    attributes=[
        {
            "name": "id",
            "type": "S",
        }
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
    metadata_table=video_metadata_table,
)

audio_transcriber_job = AudioTranscriberJob(
    "audio-transcriber-job",
    output_bucket=output_bucket,
    metadata_table=video_metadata_table,
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

stream_ingestion = StreamIngestion(
    "stream-ingestion",
    audio_transcriber_job_arn=audio_transcriber_job.job_definition_arn,
    gpu_batch_job_queue_arn=gpu_batch_job_queue.job_queue_arn,
    metadata_table=video_metadata_table,
)

# cognito userpool setup

app_user_pool = aws.cognito.UserPool(
    "AppUserPool",
    account_recovery_setting={
        "recovery_mechanisms": [
            {
                "name": "verified_email",
                "priority": 1,
            }
        ],
    },
    auto_verified_attributes=["email"],
    deletion_protection="ACTIVE",
    device_configuration={
        "challenge_required_on_new_device": True,
        "device_only_remembered_on_user_prompt": True,
    },
    email_configuration={
        "email_sending_account": "COGNITO_DEFAULT",
    },
    mfa_configuration="OPTIONAL",
    name="glowing-telegram-user-pool",
    password_policy={
        "minimum_length": 20,
        "password_history_size": 24,
        "require_lowercase": True,
        "require_numbers": True,
        "require_symbols": True,
        "require_uppercase": True,
        "temporary_password_validity_days": 1,
    },
    software_token_mfa_configuration={
        "enabled": True,
    },
    username_attributes=["email"],
    username_configuration={
        "case_sensitive": False,
    },
    verification_message_template={
        "default_email_option": "CONFIRM_WITH_CODE",
    },
    opts=pulumi.ResourceOptions(protect=True),
)

user_pool_domain = aws.cognito.UserPoolDomain(
    "AppUserPoolDomain",
    domain="glowing-telegram",
    user_pool_id=app_user_pool.id,
    opts=pulumi.ResourceOptions(protect=True),
)

user_pool_client = aws_native.cognito.UserPoolClient(
    "AppUserPoolClient",
    access_token_validity=60,
    allowed_o_auth_flows=["code"],
    allowed_o_auth_flows_user_pool_client=True,
    allowed_o_auth_scopes=[
        "aws.cognito.signin.user.admin",
        "email",
        "openid",
        "phone",
        "profile",
    ],
    auth_session_validity=3,
    callback_urls=[
        "http://localhost:5173/auth-callback",
        "https://localhost:5173/auth-callback",
        "http://localhost:5173/",
        "https://localhost:5173/",
    ],
    enable_token_revocation=True,
    explicit_auth_flows=[
        "ALLOW_REFRESH_TOKEN_AUTH",
        "ALLOW_USER_AUTH",
        "ALLOW_USER_SRP_AUTH",
    ],
    id_token_validity=60,
    logout_urls=[
        "http://localhost:5173/",
        "https://localhost:5173/",
    ],
    client_name="glowing-telegram-client",
    prevent_user_existence_errors="ENABLED",
    refresh_token_validity=5,
    supported_identity_providers=["COGNITO"],
    token_validity_units={
        "access_token": "minutes",
        "id_token": "minutes",
        "refresh_token": "days",
    },
    user_pool_id=app_user_pool.id,
    opts=pulumi.ResourceOptions(protect=True),
)

api = API(
    "gt-api",
    user_pool=app_user_pool,
    stream_ingestion_stepfunction_arn=stream_ingestion.stepfunction_arn,
    video_metadata_table=video_metadata_table,
    streams_table=streams_table,
    stream_series_table=stream_series_table,
    episodes_table=episodes_table,
    profiles_table=profiles_table,
)
