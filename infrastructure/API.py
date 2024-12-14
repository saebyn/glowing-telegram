"""
This module defines a Pulumi component resource that represents an API Gateway REST API and its associated resources, including a Lambda function and an IAM role.
"""

import pulumi
import pulumi_aws as aws
import pulumi_aws_native as aws_native
import json


class API(pulumi.ComponentResource):
    def __init__(
        self,
        name: str,
        user_pool: aws.cognito.UserPool,
        stream_ingestion_stepfunction_arn: pulumi.Output[str],
        video_metadata_table: aws.dynamodb.Table,
        streams_table: aws.dynamodb.Table,
        stream_series_table: aws.dynamodb.Table,
        episode_table: aws.dynamodb.Table,
        opts=None,
    ):
        super().__init__("glowing_telegram:infrastructure:API", name, None, opts)

        # API Gateway setup
        api = aws.apigateway.RestApi(
            "stream-ingestion-api",
            name="stream-ingestion-api",
            description="API for ingesting streams",
            endpoint_configuration={"types": "REGIONAL"},
            opts=pulumi.ResourceOptions(parent=self),
        )

        api_user_authorizer = aws.apigateway.Authorizer(
            "stream-ingestion-api-user-authorizer",
            rest_api=api.id,
            type="COGNITO_USER_POOLS",
            provider_arns=[
                user_pool.arn,
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

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
                },
            ]
        )

        aws.apigateway.RestApiPolicy(
            "stream-ingestion-api-policy",
            rest_api_id=api.id,
            policy=api_policy_document.json,
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a resource for the API
        stream_api_resource = aws.apigateway.Resource(
            "stream-ingestion-api-resource",
            rest_api=api.id,
            parent_id=api.root_resource_id,
            path_part="stream",
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a method for the API
        stream_ingestion_api_method = aws.apigateway.Method(
            "stream-ingestion-api-method",
            rest_api=api.id,
            resource_id=stream_api_resource.id,
            http_method="POST",
            authorization="COGNITO_USER_POOLS",
            authorizer_id=api_user_authorizer.id,
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a role for the API Gateway to assume when invoking the state machine
        stream_ingestion_api_gateway_role = aws_native.iam.Role(
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
                                "Resource": stream_ingestion_stepfunction_arn,
                            },
                        ],
                    },
                ),
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create an integration for the API
        st_api_integration = aws.apigateway.Integration(
            "stream-ingestion-api-integration",
            rest_api=api.id,
            resource_id=stream_api_resource.id,
            http_method=stream_ingestion_api_method.http_method,
            integration_http_method="POST",
            type="AWS",
            uri=f"arn:aws:apigateway:{aws.config.region}:states:action/StartExecution",
            request_templates={
                "application/json": pulumi.Output.all(
                    stream_ingestion_stepfunction_arn
                ).apply(
                    lambda args: json.dumps(
                        {
                            "input": "$util.escapeJavaScript($input.json('$'))",
                            "stateMachineArn": args[0],
                        }
                    )
                ),
            },
            credentials=stream_ingestion_api_gateway_role.arn,
            opts=pulumi.ResourceOptions(parent=self),
        )

        # integration response
        aws.apigateway.IntegrationResponse(
            "stream-ingestion-api-integration-response",
            rest_api=api.id,
            resource_id=stream_api_resource.id,
            http_method=stream_ingestion_api_method.http_method,
            status_code="200",
            response_templates={"application/json": "{}"},
            opts=pulumi.ResourceOptions(parent=self, depends_on=[st_api_integration]),
        )

        # Response setup for the method
        aws.apigateway.MethodResponse(
            "stream-ingestion-api-method-response",
            rest_api=api.id,
            resource_id=stream_api_resource.id,
            http_method=stream_ingestion_api_method.http_method,
            status_code="200",
            response_models={"application/json": "Empty"},
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Set up CRUD operations for the API
        crud_lambda_role = aws_native.iam.Role(
            "crud-lambda-role",
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
                    policy_name="crud-lambda-policy",
                    policy_document={
                        "Version": "2012-10-17",
                        "Statement": [
                            {
                                "Effect": "Allow",
                                "Action": [
                                    "dynamodb:BatchGetItem",
                                    "dynamodb:BatchWriteItem",
                                    "dynamodb:DeleteItem",
                                    "dynamodb:GetItem",
                                    "dynamodb:PutItem",
                                    "dynamodb:Query",
                                    "dynamodb:Scan",
                                    "dynamodb:UpdateItem",
                                ],
                                "Resource": [
                                    video_metadata_table.arn,
                                    streams_table.arn,
                                    stream_series_table.arn,
                                    episode_table.arn,
                                ],
                            },
                        ],
                    },
                ),
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        crud_lambda = aws.lambda_.Function(
            "crud-lambda",
            runtime=aws.lambda_.Runtime.CUSTOM_AL2023,
            timeout=15 * 60,
            code=pulumi.AssetArchive(
                {"bootstrap": pulumi.FileAsset("../target/debug/crud_api")}
            ),
            tracing_config={"mode": "Active"},
            handler="doesnt.matter",
            role=crud_lambda_role.arn,
            environment={
                "variables": {
                    "VIDEO_METADATA_TABLE": video_metadata_table.name,
                    "STREAMS_TABLE": streams_table.name,
                    "STREAM_SERIES_TABLE": stream_series_table.name,
                    "EPISODE_TABLE": episode_table.name,
                }
            },
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a resource for the API
        crud_api_resource = aws.apigateway.Resource(
            "crud-api-resource",
            rest_api=api.id,
            parent_id=api.root_resource_id,
            path_part="records",
            opts=pulumi.ResourceOptions(parent=self),
        )

        crud_api_records_resource = aws.apigateway.Resource(
            "crud-api-records-resource",
            rest_api=api.id,
            parent_id=crud_api_resource.id,
            path_part="{recordType}",
            opts=pulumi.ResourceOptions(parent=self),
        )

        crud_api_record_resource = aws.apigateway.Resource(
            "crud-api-record-resource",
            rest_api=api.id,
            parent_id=crud_api_records_resource.id,
            path_part="{recordId}",
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Data defining the API Gateway resources and methods, and how they integrate with the Lambda function
        api_definition = [
            {
                "name": "list-records",
                "path": "/records/{recordType}",
                "resource": crud_api_records_resource,
                "method": "GET",
                "request_parameters": {
                    "method.request.path.recordType": True,
                    "method.request.querystring.filter": False,
                    "method.request.querystring.cursor": False,
                    "method.request.querystring.limit": False,
                },
            },
            {
                "name": "create-record",
                "path": "/records/{recordType}",
                "resource": crud_api_records_resource,
                "method": "POST",
                "request_parameters": {
                    "method.request.path.recordType": True,
                },
            },
            {
                "name": "get-record",
                "path": "/records/{recordType}/{recordId}",
                "resource": crud_api_record_resource,
                "method": "GET",
                "request_parameters": {
                    "method.request.path.recordType": True,
                    "method.request.path.recordId": True,
                },
            },
            {
                "name": "update-record",
                "path": "/records/{recordType}/{recordId}",
                "resource": crud_api_record_resource,
                "method": "PUT",
                "request_parameters": {
                    "method.request.path.recordType": True,
                    "method.request.path.recordId": True,
                },
            },
            {
                "name": "delete-record",
                "path": "/records/{recordType}/{recordId}",
                "resource": crud_api_record_resource,
                "method": "DELETE",
                "request_parameters": {
                    "method.request.path.recordType": True,
                    "method.request.path.recordId": True,
                },
            },
        ]

        mapping_fields = {
            "recordType": "method.request.path.recordType",
            "recordId": "method.request.path.recordId",
            "filter": "method.request.querystring.filter",
            "cursor": "method.request.querystring.cursor",
            "limit": "method.request.querystring.limit",
        }

        request_template = json.dumps(
            {
                "application/json": json.dumps(
                    {
                        "payload": "$input.json('$')",
                        "method": "$context.httpMethod",
                        "resource": "$input.params('recordType')",
                        "record_id": "$input.params('recordId')",
                        "query": {
                            "filter": "$input.params('filter')",
                            "cursor": "$input.params('cursor')",
                            "limit": "$input.params('limit')",
                        },
                    }
                )
            }
        )

        # Create methods and integrations for the crud lambda
        for definition in api_definition:
            method = aws.apigateway.Method(
                f"crud-api-{definition['name']}-{definition['method'].lower()}",
                rest_api=api.id,
                resource_id=definition["resource"].id,
                http_method=definition["method"],
                authorization="COGNITO_USER_POOLS",
                authorizer_id=api_user_authorizer.id,
                request_parameters=definition["request_parameters"],
                opts=pulumi.ResourceOptions(parent=self),
            )

            integration = aws.apigateway.Integration(
                f"crud-api-integration-{definition['name']}-{definition['method'].lower()}",
                rest_api=api.id,
                resource_id=definition["resource"].id,
                http_method=method.http_method,
                integration_http_method="POST",
                type="AWS_PROXY",
                uri=crud_lambda.invoke_arn,
                request_templates={"application/json": request_template},
                credentials=crud_lambda_role.arn,
                opts=pulumi.ResourceOptions(parent=self),
            )

            aws.apigateway.IntegrationResponse(
                f"crud-api-integration-response-{definition['name']}-{definition['method'].lower()}",
                rest_api=api.id,
                resource_id=definition["resource"].id,
                http_method=method.http_method,
                status_code="200",
                # TODO
                response_templates={"application/json": "{}"},
                opts=pulumi.ResourceOptions(parent=self, depends_on=[integration]),
            )

            aws.apigateway.MethodResponse(
                f"crud-api-method-response-{definition['name']}-{definition['method'].lower()}",
                rest_api=api.id,
                resource_id=definition["resource"].id,
                http_method=method.http_method,
                status_code="200",
                response_models={"application/json": "Empty"},
                opts=pulumi.ResourceOptions(parent=self),
            )

        # Trigger a deployment of the API when we do 'pulumi up'
        # TODO do this the right way with a stage
        # aws.apigateway.Deployment(
        #    "stream-ingestion-api-deployment",
        #    rest_api=api.id,
        #    stage_name="tst",
        #    opts=pulumi.ResourceOptions(parent=self),
        # ``)
