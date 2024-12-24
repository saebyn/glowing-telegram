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
        video_archive_bucket: aws.s3.Bucket,
        opts=None,
    ):
        super().__init__(
            "glowing_telegram:infrastructure:StreamIngestion", name, None, opts
        )

        # Create secret for OpenAI API key
        openai_secret = aws.secretsmanager.Secret(
            f"{name}-openai-secret",
            name=f"{name}-openai-secret",
            description="OpenAI API key",
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a Lambda execution role
        summarize_transcription_lambda_role = aws_native.iam.Role(
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
            policies=[
                aws_native.iam.RolePolicyArgs(
                    policy_name=f"{name}-summarize-transcription-lambda-policy",
                    policy_document={
                        "Version": "2012-10-17",
                        "Statement": [
                            # DynamoDB permissions
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "dynamodb:PutItem",
                                    "dynamodb:GetItem",
                                    "dynamodb:UpdateItem",
                                ],
                                "Resource": metadata_table.arn,
                            },
                            # Allow X-Ray tracing
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
                            # Allow getting the OpenAI secret
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "secretsmanager:GetSecretValue",
                                ],
                                "Resource": openai_secret.arn,
                            },
                        ],
                    },
                )
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a Lambda function
        summarize_transcription_lambda = aws.lambda_.Function(
            f"{name}-summarize_transcription_lambda",
            runtime=aws.lambda_.Runtime.CUSTOM_AL2023,
            timeout=15 * 60,
            code=pulumi.AssetArchive(
                {
                    "bootstrap": pulumi.FileAsset(
                        "../target/debug/summarize_transcription"
                    )
                }
            ),
            tracing_config={"mode": "Active"},
            handler="doesnt.matter",
            role=summarize_transcription_lambda_role.arn,
            environment={
                "variables": {
                    "OPENAI_SECRET_ARN": openai_secret.arn,
                    "METADATA_TABLE_NAME": metadata_table.name,
                    "OPENAI_MODEL": "gpt-4o-2024-08-06",
                    "OPENAI_INSTRUCTIONS": """
Generate a detailed summary report for the given transcript of a 20-minute video, using the provided context summary of preceding videos to enhance continuity and depth.

The summary you generate must be not only informational for content review but also reusable for future summarization and reference purposes. Combine the details from the current video with the larger context of the ongoing series to identify recurring themes, connections, and key points.

# Steps
1. **Analyze the Transcript**: Read the 20-minute transcript thoroughly to capture major discussion points, arguments, examples, questions, and any pivotal moments or insights, and noting the time periods of each.
2. **Incorporate Preceding Context**: Use the summary of the preceding videos to identify overarching topics, common themes, recurring elements, and key progressions in the narrative.
3. **Extract Key Points**: Highlight:
   - The main topics covered in the current video.
   - Key arguments or perspectives.
   - Examples or anecdotes that have importance.
   - How the discussion connects to or extends previous episodes.
4. **Generate the Output**:
   - Create a high-level summary of the current video.
   - Note connections to previous videos, showing continuity of ideas and context where applicable.
   - Identify questions introduced or resolved, transitions in focus, or shifts from the previous video.
   - Highlight significant new points or insights and how they enhance the larger theme.
   - Review any errors or inconsistencies in the transcript that need clarification or correction (attentions).
   - Identify any gaffs or issues that might require further investigation or follow-up (transcript errors).

# Notes 
- Ensure continuity between videos by emphasizing the ongoing build of ideas.
- Focus on the usefulness of the `summary_context` in shaping future summaries, noting key phrases, themes, or topics that might resurface or require revisiting.""",
                }
            },
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
                                    "s3:ListObjects",
                                ],
                                "Resource": [
                                    video_archive_bucket.arn,
                                ],
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "batch:SubmitJob",
                                    "batch:DescribeJobs",
                                ],
                                "Resource": [
                                    gpu_batch_job_queue_arn,
                                    audio_transcriber_job_arn,
                                ],
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "batch:TerminateJob",
                                ],
                                "Resource": "*",
                            },
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "events:PutTargets",
                                    "events:PutRule",
                                    "events:DescribeRule",
                                ],
                                "Resource": [
                                    f"arn:aws:events:{aws.config.region}:{aws.get_caller_identity().account_id}:rule/StepFunctions*",
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
                                    "dynamodb:UpdateItem",
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

        self.stepfunction_arn = my_state_machine.arn

        self.register_outputs(
            {
                "stepfunction_arn": my_state_machine.arn,
            }
        )
