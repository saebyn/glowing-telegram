"""
This module defines a Pulumi component resource that does stream-level ingestion and processing of video and audio data, transcription, and
summarization.
"""

import pulumi
import pulumi_aws as aws
import pulumi_aws_native as aws_native
import json


class StreamIngestion(pulumi.ComponentResource):
    def __init__(
        self,
        name: str,
        audio_transcriber_job_arn: str,
        gpu_batch_job_queue_arn: str,
        metadata_table: aws.dynamodb.Table,
        opts=None,
    ):
        super().__init__(
            "glowing_telegram:infrastructure:StreamIngestion", name, None, opts
        )

        lambda_role = aws_native.iam.Role(
            f"{name}-summarize-transcription-lambda-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "lambda.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            managed_policy_arns=[
                "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a Lambda function
        summarize_transcription_lambda = aws.lambda_.Function(
            f"{name}-summarize_transcription_lambda",
            runtime=aws.lambda_.Runtime.CUSTOM_AL2023,
            code=pulumi.AssetArchive(
                {
                    "bootstrap": pulumi.FileAsset(
                        "../target/debug/summarize_transcription"
                    )
                }
            ),
            tracing_config={"mode": "Active"},
            handler="doesnt.matter",
            role=lambda_role.arn,
            opts=pulumi.ResourceOptions(parent=self),
        )

        state_machine_definition = "{}"
        with open("./stepfunction.json") as f:
            state_machine_definition = f.read()

        state_machine_role = aws_native.iam.Role(
            f"{name}-state-machine-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "states.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            policies=[
                aws_native.iam.RolePolicyArgs(
                    policy_name=f"{name}-state-machine-policy",
                    policy_document={
                        "Version": "2012-10-17",
                        "Statement": [
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "batch:SubmitJob",
                                    "batch:DescribeJobs",
                                    "batch:TerminateJob",
                                ],
                                "Resource": "*",  # TODO: constrain to the job queue and job definition
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "events:PutTargets",
                                    "events:PutRule",
                                    "events:DescribeRule",
                                ],
                                "Resource": [
                                    "arn:aws:events:us-west-2:159222827421:rule/StepFunctionsGetEventsForBatchJobsRule"  # TODO use region and account id
                                ],
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "lambda:InvokeFunction",
                                ],
                                "Resource": summarize_transcription_lambda.qualified_arn,
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "dynamodb:PutItem",
                                    "dynamodb:GetItem",
                                ],
                                "Resource": metadata_table.arn,
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "xray:PutTraceSegments",
                                    "xray:PutTelemetryRecords",
                                    "xray:GetSamplingRules",
                                    "xray:GetSamplingTargets",
                                ],
                                "Resource": ["*"],
                            },
                        ],
                    },
                )
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        my_state_machine = aws_native.stepfunctions.StateMachine(
            f"{name}-state-machine",
            definition_string=state_machine_definition,
            definition_substitutions={
                "metadataTableName": metadata_table.name,
                "summarizeTranscriptionFunctionArn": summarize_transcription_lambda.qualified_arn,
                "audioTranscriberJobQueueArn": gpu_batch_job_queue_arn,
                "audioTranscriberJobDefinitionArn": audio_transcriber_job_arn,
            },
            role_arn=state_machine_role.arn,
            tracing_configuration={"enabled": True},
            opts=pulumi.ResourceOptions(parent=self),
        )
