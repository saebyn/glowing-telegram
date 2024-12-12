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

episode_table = aws.dynamodb.Table(
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

stream_ingestion = StreamIngestion(
    "stream-ingestion",
    audio_transcriber_job_arn=audio_transcriber_job.job_definition_arn,
    gpu_batch_job_queue_arn=gpu_batch_job_queue.job_queue_arn,
    metadata_table=metadata_table,
)

###
# Some demo code to show how to use API gateway to trigger the state machine
# This is not a complete implementation, and is not safe for production use
###

# API Gateway setup
api = aws.apigateway.RestApi(
    "stream-ingestion-api",
    name="stream-ingestion-api",
    description="API for ingesting streams",
    endpoint_configuration={"types": "REGIONAL"},
)

# set a resource policy for the API that allows one IP address to access the API
my_ip = pulumi.Config().require_secret("my_ip")

api_policy_document = aws.iam.get_policy_document_output(
    statements=[
        {
            "effect": "Allow",
            "principals": [
                {
                    "type": "AWS",
                    "identifiers": ["*"],
                }
            ],
            "actions": ["execute-api:Invoke"],
            "resources": [api.execution_arn.apply(lambda arn: f"{arn}/*/*/*")],
            "conditions": [
                {
                    "test": "IpAddress",
                    "values": [my_ip],
                    "variable": "aws:SourceIp",
                }
            ],
        },
    ]
)


api_policy = aws.apigateway.RestApiPolicy(
    "stream-ingestion-api-policy",
    rest_api_id=api.id,
    policy=api_policy_document.json,
)

# Create a resource for the API
resource = aws.apigateway.Resource(
    "stream-ingestion-api-resource",
    rest_api=api.id,
    parent_id=api.root_resource_id,
    path_part="stream",
)

# Create a method for the API
method = aws.apigateway.Method(
    "stream-ingestion-api-method",
    rest_api=api.id,
    resource_id=resource.id,
    http_method="POST",
    authorization="NONE",
)


# Create a role for the API Gateway to assume when invoking the state machine
api_gateway_role = aws_native.iam.Role(
    "api-gateway-role",
    assume_role_policy_document={
        "Version": "2012-10-17",
        "Statement": [
            {
                "Effect": "Allow",
                "Principal": {
                    "Service": "apigateway.amazonaws.com",
                },
                "Action": "sts:AssumeRole",
            },
        ],
    },
    policies=[
        aws_native.iam.RolePolicyArgs(
            policy_name="api-gateway-policy",
            policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Action": ["states:StartExecution"],
                        "Resource": stream_ingestion.stepfunction_arn,
                    },
                ],
            },
        ),
    ],
)

# Create an integration for the API
integration = aws.apigateway.Integration(
    "stream-ingestion-api-integration",
    rest_api=api.id,
    resource_id=resource.id,
    http_method=method.http_method,
    integration_http_method="POST",
    type="AWS",
    uri=f"arn:aws:apigateway:{aws.config.region}:states:action/StartExecution",
    request_templates={
        "application/json": pulumi.Output.all(stream_ingestion.stepfunction_arn).apply(
            lambda args: json.dumps(
                {
                    "input": "$util.escapeJavaScript($input.json('$'))",
                    "stateMachineArn": args[0],
                }
            )
        ),
    },
    credentials=api_gateway_role.arn,
)

# integration response
integration_response = aws.apigateway.IntegrationResponse(
    "stream-ingestion-api-integration-response",
    rest_api=api.id,
    resource_id=resource.id,
    http_method=method.http_method,
    status_code="200",
    response_templates={"application/json": "{}"},
)

# Response setup for the method
response = aws.apigateway.MethodResponse(
    "stream-ingestion-api-method-response",
    rest_api=api.id,
    resource_id=resource.id,
    http_method=method.http_method,
    status_code="200",
    response_models={"application/json": "Empty"},
)
