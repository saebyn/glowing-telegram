import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as apigwv2 from 'aws-cdk-lib/aws-apigatewayv2';
import type * as cognito from 'aws-cdk-lib/aws-cognito';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as lambdaPython from '@aws-cdk/aws-lambda-python-alpha';
import * as logs from 'aws-cdk-lib/aws-logs';
import { WebSocketLambdaIntegration } from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import { WebSocketLambdaAuthorizer } from 'aws-cdk-lib/aws-apigatewayv2-authorizers';
import { DynamoEventSource } from 'aws-cdk-lib/aws-lambda-event-sources';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from './util/serviceLambda';

interface WebSocketAPIConstructProps {
  userPool: cognito.IUserPool;
  tasksTable: dynamodb.ITable;
  userPoolClient: cognito.IUserPoolClient;
  domainName: string;
}

export default class WebSocketAPIConstruct extends Construct {
  public readonly webSocketApi: apigwv2.WebSocketApi;
  public readonly webSocketStage: apigwv2.WebSocketStage;
  public readonly connectionsTable: dynamodb.Table;

  constructor(scope: Construct, id: string, props: WebSocketAPIConstructProps) {
    super(scope, id);

    // Create a DynamoDB table to store WebSocket connections
    this.connectionsTable = new dynamodb.Table(this, 'ConnectionsTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: {
        name: 'connectionId',
        type: dynamodb.AttributeType.STRING,
      },
      timeToLiveAttribute: 'ttl',
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Add user_id index to look up connections by user
    this.connectionsTable.addGlobalSecondaryIndex({
      indexName: 'user_id-index',
      partitionKey: { name: 'user_id', type: dynamodb.AttributeType.STRING },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // Create the WebSocket API
    this.webSocketApi = new apigwv2.WebSocketApi(this, 'TasksWebSocketApi', {
      apiName: 'TasksWebSocketApi',
      description: 'WebSocket API for task updates',
    });

    // Create a production stage for the WebSocket API
    this.webSocketStage = new apigwv2.WebSocketStage(
      this,
      'WebSocketProdStage',
      {
        webSocketApi: this.webSocketApi,
        stageName: 'prod',
        autoDeploy: true,
      },
    );

    // Create explicit log group for authorizer lambda
    const authorizerLogGroup = new logs.LogGroup(this, 'AuthorizerLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/lambda/websocket-authorizer`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Create the authorizer Lambda for WebSocket API (Python)
    const authorizerLambda = new lambdaPython.PythonFunction(
      this,
      'AuthorizerLambda',
      {
        runtime: lambda.Runtime.PYTHON_3_11,
        tracing: lambda.Tracing.ACTIVE,
        entry: 'lib/websocketAuthorizer',
        index: 'main.py',
        handler: 'handler',
        timeout: cdk.Duration.seconds(10),
        environment: {
          USER_POOL_ID: props.userPool.userPoolId,
          USER_POOL_CLIENT_ID: props.userPoolClient.userPoolClientId,
        },
        logGroup: authorizerLogGroup,
        loggingFormat: lambda.LoggingFormat.JSON,
      },
    );

    // Create explicit log group for connect handler
    const connectLogGroup = new logs.LogGroup(this, 'ConnectLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/lambda/websocket-connect`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Create Lambda for handling WebSocket connect events (Python)
    const connectHandler = new lambda.Function(this, 'ConnectHandler', {
      runtime: lambda.Runtime.PYTHON_3_11,
      code: lambda.Code.fromInline(`
import json
import os
import time
import boto3
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

def handler(event, context):
    logger.info(f"Connect event: {json.dumps(event)}")
    
    connection_id = event.get('requestContext', {}).get('connectionId')
    if not connection_id:
        logger.error("No connectionId found in event")
        return {'statusCode': 400, 'body': 'No connectionId provided'}
    
    # Extract user information from the authorizer context
    authorizer = event.get('requestContext', {}).get('authorizer', {})
    user_id = authorizer.get('userId')
    
    if not user_id:
        logger.error("No userId found in authorizer context")
        return {'statusCode': 401, 'body': 'Unauthorized'}
    
    try:
        ttl = int(time.time()) + (24 * 60 * 60)  # 24 hours in seconds
        
        connections_table.put_item(
            Item={
                'connectionId': connection_id,
                'user_id': user_id,
                'ttl': ttl,
                'connected_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
            }
        )
        
        logger.info(f"Connection {connection_id} for user {user_id} stored successfully")
        return {'statusCode': 200, 'body': 'Connected'}
    
    except Exception as e:
        logger.error(f"Error storing connection: {str(e)}")
        return {'statusCode': 500, 'body': 'Failed to connect'}
      `),
      handler: 'index.handler',
      timeout: cdk.Duration.seconds(30),
      environment: {
        CONNECTIONS_TABLE: this.connectionsTable.tableName,
      },
      logGroup: connectLogGroup,
      loggingFormat: lambda.LoggingFormat.JSON,
    });

    // Create explicit log group for disconnect handler
    const disconnectLogGroup = new logs.LogGroup(this, 'DisconnectLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/lambda/websocket-disconnect`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Create Lambda for handling WebSocket disconnect events (Python)
    const disconnectHandler = new lambda.Function(this, 'DisconnectHandler', {
      runtime: lambda.Runtime.PYTHON_3_11,
      code: lambda.Code.fromInline(`
import json
import os
import boto3
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

def handler(event, context):
    logger.info(f"Disconnect event: {json.dumps(event)}")
    
    connection_id = event.get('requestContext', {}).get('connectionId')
    if not connection_id:
        logger.error("No connectionId found in event")
        return {'statusCode': 400, 'body': 'No connectionId provided'}
    
    try:
        # Remove the connection from DynamoDB
        connections_table.delete_item(
            Key={
                'connectionId': connection_id
            }
        )
        
        logger.info(f"Connection {connection_id} removed successfully")
        return {'statusCode': 200, 'body': 'Disconnected'}
    
    except Exception as e:
        logger.error(f"Error removing connection: {str(e)}")
        return {'statusCode': 500, 'body': 'Failed to disconnect'}
      `),
      handler: 'index.handler',
      timeout: cdk.Duration.seconds(30),
      environment: {
        CONNECTIONS_TABLE: this.connectionsTable.tableName,
      },
      logGroup: disconnectLogGroup,
      loggingFormat: lambda.LoggingFormat.JSON,
    });

    // Create explicit log group for task change handler
    const taskChangeLogGroup = new logs.LogGroup(this, 'TaskChangeLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/lambda/websocket-task-change`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Create Lambda for handling task changes and publishing to WebSocket (Python)
    const taskChangeHandler = new lambda.Function(this, 'TaskChangeHandler', {
      runtime: lambda.Runtime.PYTHON_3_11,
      code: lambda.Code.fromInline(`
import json
import os
import boto3
import base64
import logging
from botocore.exceptions import ClientError

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

# Initialize API Gateway Management API client
# The endpoint will be set at deployment time
endpoint = os.environ.get('WEBSOCKET_ENDPOINT', '')
if endpoint.startswith('wss://'):
    endpoint = endpoint.replace('wss://', 'https://')
    
api_client = boto3.client('apigatewaymanagementapi', endpoint_url=endpoint) if endpoint else None

def handler(event, context):
    logger.info(f"Task change event received")
    
    if not api_client:
        logger.error(f"No valid WebSocket endpoint configured: {os.environ.get('WEBSOCKET_ENDPOINT')}")
        return
    
    # Process each record in the DynamoDB stream
    for record in event.get('Records', []):
        if record.get('eventName') in ['MODIFY', 'INSERT']:
            try:
                # Extract the new image of the task
                if 'dynamodb' in record and 'NewImage' in record['dynamodb']:
                    new_task = deserialize_dynamodb_item(record['dynamodb']['NewImage'])
                    old_task = deserialize_dynamodb_item(record['dynamodb'].get('OldImage'))
                    handle_task_change(new_task, old_task)
            except Exception as e:
                logger.error(f"Error processing record: {str(e)}")
    
def handle_task_change(task, old_task):
    if not task or 'id' not in task:
        logger.warning("No valid task data found in the record")
        return
    
    # Get the user ID associated with this task
    user_id = task.get('user_id')
    if not user_id:
        logger.warning(f"No user_id found for task: {task.get('id')}")
        return
    
    try:
        # Find all active connections for this user
        connections = find_connections_for_user(user_id)
        
        if not connections:
            logger.info(f"No active connections found for user {user_id}")
            return
        
        # Prepare the payload to send to clients
        payload = json.dumps({
            'type': 'TASK_UPDATE',
            'task': task,
            'old_status': old_task.get('status') if old_task else None,
        })
        
        # Send the update to all connections for this user
        for connection_id in connections:
            try:
                api_client.post_to_connection(
                    ConnectionId=connection_id,
                    Data=payload
                )
                logger.info(f"Successfully sent update to connection {connection_id}")
            except ClientError as e:
                # Handle stale connections
                if e.response.get('Error', {}).get('Code') == 'GoneException':
                    logger.info(f"Connection {connection_id} is stale, removing it")
                    remove_connection(connection_id)
                else:
                    logger.error(f"Error sending message to connection {connection_id}: {str(e)}")
    
    except Exception as e:
        logger.error(f"Error processing task change: {str(e)}")

def find_connections_for_user(user_id):
    try:
        response = connections_table.query(
            IndexName='user_id-index',
            KeyConditionExpression='user_id = :userId',
            ExpressionAttributeValues={
                ':userId': user_id
            }
        )
        
        return [item.get('connectionId') for item in response.get('Items', [])]
    
    except Exception as e:
        logger.error(f"Error querying connections for user: {str(e)}")
        return []

def remove_connection(connection_id):
    try:
        connections_table.delete_item(
            Key={
                'connectionId': connection_id
            }
        )
    except Exception as e:
        logger.error(f"Error removing stale connection {connection_id}: {str(e)}")

def deserialize_dynamodb_item(item):
    if not item:
        return None
    
    # Convert DynamoDB types to Python types
    result = {}
    for key, value in item.items():
        if 'S' in value:
            result[key] = value['S']
        elif 'N' in value:
            result[key] = float(value['N']) if '.' in value['N'] else int(value['N'])
        elif 'BOOL' in value:
            result[key] = value['BOOL']
        elif 'M' in value:
            result[key] = deserialize_dynamodb_item(value['M'])
        elif 'L' in value:
            result[key] = [deserialize_dynamodb_item({'M': item}) if 'M' in item else None for item in value['L']]
        # Add other types as needed
    
    return result
      `),
      handler: 'index.handler',
      timeout: cdk.Duration.seconds(30),
      environment: {
        CONNECTIONS_TABLE: this.connectionsTable.tableName,
        WEBSOCKET_ENDPOINT: this.webSocketStage.url,
      },
      logGroup: taskChangeLogGroup,
      loggingFormat: lambda.LoggingFormat.JSON,
    });

    // Grant permissions to the Lambda functions
    this.connectionsTable.grantReadWriteData(connectHandler);
    this.connectionsTable.grantReadWriteData(disconnectHandler);
    this.connectionsTable.grantReadData(taskChangeHandler);

    taskChangeHandler.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          'execute-api:PostToConnection',
          'execute-api:ManageConnections',
          'execute-api:Invoke',
        ],
        resources: [
          `arn:aws:execute-api:${cdk.Stack.of(this).region}:${cdk.Stack.of(this).account}:${this.webSocketApi.apiId}/*`,
        ],
        effect: iam.Effect.ALLOW,
      }),
    );

    // Create the IAM role for API Gateway to invoke the authorizer
    const authorizerInvokeRole = new iam.Role(this, 'AuthorizerInvokeRole', {
      assumedBy: new iam.ServicePrincipal('apigateway.amazonaws.com'),
    });

    // Grant the role permission to invoke the authorizer Lambda
    authorizerLambda.grantInvoke(authorizerInvokeRole);

    // Create the WebSocket API Authorizer
    const authorizer = new WebSocketLambdaAuthorizer(
      'WebSocketAuthorizer',
      authorizerLambda,
      {
        identitySource: ['route.request.querystring.token'],
      },
    );

    this.webSocketApi.addRoute('$connect', {
      integration: new WebSocketLambdaIntegration(
        'ConnectIntegration',
        connectHandler,
      ),
      authorizer,
    });

    this.webSocketApi.addRoute('$disconnect', {
      integration: new WebSocketLambdaIntegration(
        'DisconnectIntegration',
        disconnectHandler,
      ),
    });

    // Connect the task change handler to the DynamoDB stream
    taskChangeHandler.addEventSource(
      new DynamoEventSource(props.tasksTable, {
        startingPosition: lambda.StartingPosition.LATEST,
      }),
    );
  }
}
