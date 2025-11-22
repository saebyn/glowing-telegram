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
  streamWidgetsTable: dynamodb.ITable;
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

    // Create log group for WebSocket API access logging
    const webSocketApiLogGroup = new logs.LogGroup(this, 'WebSocketApiLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/apigateway/websocket-api`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
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

    // Configure access logging for the WebSocket stage
    const webSocketStage = this.webSocketStage.node.defaultChild as apigwv2.CfnStage;
    if (webSocketStage) {
      webSocketStage.accessLogSettings = {
        destinationArn: webSocketApiLogGroup.logGroupArn,
        format: JSON.stringify({
          requestId: '$context.requestId',
          ip: '$context.identity.sourceIp',
          requestTime: '$context.requestTime',
          eventType: '$context.eventType',
          routeKey: '$context.routeKey',
          status: '$context.status',
          connectionId: '$context.connectionId',
          integrationError: '$context.integrationErrorMessage',
        }),
      };
    }

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
          STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
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
    
    # Extract auth information from the authorizer context
    authorizer = event.get('requestContext', {}).get('authorizer', {})
    user_id = authorizer.get('userId', '')
    auth_type = authorizer.get('authType', 'FullAccess')
    widget_id = authorizer.get('widgetId', '')
    
    if not user_id and not widget_id:
        logger.error("No userId or widgetId found in authorizer context")
        return {'statusCode': 401, 'body': 'Unauthorized'}
    
    try:
        ttl = int(time.time()) + (24 * 60 * 60)  # 24 hours in seconds
        
        item = {
            'connectionId': connection_id,
            'ttl': ttl,
            'connected_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
            'authType': auth_type
        }
        
        if user_id:
            item['user_id'] = user_id
        if widget_id:
            item['widgetId'] = widget_id
        
        connections_table.put_item(Item=item)
        
        logger.info(f"Connection {connection_id} stored successfully (authType: {auth_type})")
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

    // Create Lambda for handling WebSocket messages (Python)
    const messageHandler = new lambda.Function(this, 'MessageHandler', {
      runtime: lambda.Runtime.PYTHON_3_11,
      code: lambda.Code.fromInline(`
import json
import os
import boto3
import logging
from botocore.exceptions import ClientError

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize clients
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])
widgets_table = dynamodb.Table(os.environ['STREAM_WIDGETS_TABLE'])

# Initialize API Gateway Management API client
endpoint = os.environ.get('WEBSOCKET_ENDPOINT', '')
if not endpoint:
    logger.warning("WEBSOCKET_ENDPOINT environment variable not set")
elif endpoint.startswith('wss://'):
    endpoint = endpoint.replace('wss://', 'https://')

api_client = boto3.client('apigatewaymanagementapi', endpoint_url=endpoint) if endpoint else None

# Note: Subscriptions should be tracked in DynamoDB for production
# In-memory tracking shown here for MVP demonstration only
# Each Lambda invocation has isolated memory, so subscriptions won't persist
# across invocations or be shared between concurrent executions

def handler(event, context):
    logger.info(f"Message event: {json.dumps(event)}")
    
    connection_id = event.get('requestContext', {}).get('connectionId')
    if not connection_id:
        logger.error("No connectionId found")
        return {'statusCode': 400, 'body': 'Bad Request'}
    
    # Get connection info
    try:
        conn_response = connections_table.get_item(Key={'connectionId': connection_id})
        connection = conn_response.get('Item')
        if not connection:
            logger.error(f"Connection {connection_id} not found")
            return {'statusCode': 404, 'body': 'Connection not found'}
    except Exception as e:
        logger.error(f"Error fetching connection: {str(e)}")
        return {'statusCode': 500, 'body': 'Internal error'}
    
    # Parse message
    body = event.get('body', '{}')
    try:
        message = json.loads(body)
    except json.JSONDecodeError:
        logger.error("Invalid JSON in message body")
        return {'statusCode': 400, 'body': 'Invalid JSON'}
    
    message_type = message.get('type')
    
    if message_type == 'WIDGET_SUBSCRIBE':
        return handle_subscribe(connection_id, connection, message)
    elif message_type == 'WIDGET_UNSUBSCRIBE':
        return handle_unsubscribe(connection_id, message)
    elif message_type == 'WIDGET_ACTION':
        return handle_action(connection_id, connection, message)
    else:
        logger.warning(f"Unknown message type: {message_type}")
        return {'statusCode': 400, 'body': 'Unknown message type'}

def handle_subscribe(connection_id, connection, message):
    widget_id = message.get('widgetId')
    if not widget_id:
        return {'statusCode': 400, 'body': 'Missing widgetId'}
    
    # Validate access
    auth_type = connection.get('authType', 'FullAccess')
    
    if auth_type == 'WidgetAccess':
        # Widget token can only access its own widget
        if connection.get('widgetId') != widget_id:
            logger.warning(f"Widget access denied: {connection.get('widgetId')} != {widget_id}")
            return {'statusCode': 403, 'body': 'Forbidden'}
    elif auth_type == 'FullAccess':
        # User JWT can access any widget they own
        try:
            # Query by partition key only (id), since we don't have the sort key (title)
            query_response = widgets_table.query(
                KeyConditionExpression='id = :id',
                ExpressionAttributeValues={':id': widget_id},
                Limit=1
            )
            items = query_response.get('Items', [])
            widget = items[0] if items else None
            
            if not widget:
                return {'statusCode': 404, 'body': 'Widget not found'}
            
            if widget.get('user_id') != connection.get('user_id'):
                logger.warning(f"User {connection.get('user_id')} does not own widget {widget_id}")
                return {'statusCode': 403, 'body': 'Forbidden'}
        except Exception as e:
            logger.error(f"Error fetching widget: {str(e)}")
            return {'statusCode': 500, 'body': 'Internal error'}
    
    # Add widget_id to the connection's subscribed_widgets set
    try:
        connections_table.update_item(
            Key={'connectionId': connection_id},
            UpdateExpression='ADD subscribed_widgets :widget_id',
            ExpressionAttributeValues={
                ':widget_id': {widget_id}
            }
        )
        logger.info(f"Connection {connection_id} subscribed to widget {widget_id}")
    except Exception as e:
        logger.error(f"Error updating subscription: {str(e)}")
        return {'statusCode': 500, 'body': 'Failed to subscribe'}
    
    # Fetch and send initial state
    try:
        widget_response = widgets_table.query(
            KeyConditionExpression='id = :id',
            ExpressionAttributeValues={':id': widget_id},
            Limit=1
        )
        items = widget_response.get('Items', [])
        if items:
            widget = items[0]
            send_message(connection_id, {
                'type': 'WIDGET_INITIAL_STATE',
                'widgetId': widget_id,
                'widget': widget
            })
        else:
            logger.warning(f"Widget {widget_id} not found for initial state")
    except Exception as e:
        logger.error(f"Error sending initial state: {str(e)}")
    
    return {'statusCode': 200, 'body': 'Subscribed'}

def handle_unsubscribe(connection_id, message):
    widget_id = message.get('widgetId')
    if not widget_id:
        return {'statusCode': 400, 'body': 'Missing widgetId'}
    
    # Remove widget_id from the connection's subscribed_widgets set
    try:
        connections_table.update_item(
            Key={'connectionId': connection_id},
            UpdateExpression='DELETE subscribed_widgets :widget_id',
            ExpressionAttributeValues={
                ':widget_id': {widget_id}
            }
        )
        logger.info(f"Connection {connection_id} unsubscribed from widget {widget_id}")
    except Exception as e:
        logger.error(f"Error removing subscription: {str(e)}")
        return {'statusCode': 500, 'body': 'Failed to unsubscribe'}
    
    return {'statusCode': 200, 'body': 'Unsubscribed'}

def handle_action(connection_id, connection, message):
    # Widget access is read-only
    auth_type = connection.get('authType', 'FullAccess')
    if auth_type == 'WidgetAccess':
        return {'statusCode': 403, 'body': 'Read-only access'}
    
    widget_id = message.get('widgetId')
    action = message.get('action')
    payload = message.get('payload', {})
    
    if not widget_id or not action:
        return {'statusCode': 400, 'body': 'Missing widgetId or action'}
    
    # Verify ownership
    try:
        widget_response = widgets_table.query(
            KeyConditionExpression='id = :id',
            ExpressionAttributeValues={':id': widget_id},
            Limit=1
        )
        items = widget_response.get('Items', [])
        if not items:
            return {'statusCode': 404, 'body': 'Widget not found'}
        
        widget = items[0]
        if widget.get('user_id') != connection.get('user_id'):
            return {'statusCode': 403, 'body': 'Forbidden'}
        
        # TODO: Execute action based on widget type and action name
        # For now, just send success response
        
        send_message(connection_id, {
            'type': 'WIDGET_ACTION_RESPONSE',
            'widgetId': widget_id,
            'action': action,
            'success': True
        })
        
        return {'statusCode': 200, 'body': 'Action executed'}
    except Exception as e:
        logger.error(f"Error executing action: {str(e)}")
        send_message(connection_id, {
            'type': 'WIDGET_ACTION_RESPONSE',
            'widgetId': widget_id,
            'action': action,
            'success': False,
            'error': str(e)
        })
        return {'statusCode': 500, 'body': 'Internal error'}

def send_message(connection_id, message):
    if not api_client:
        logger.error("API Gateway Management API client not initialized")
        return
    
    try:
        api_client.post_to_connection(
            ConnectionId=connection_id,
            Data=json.dumps(message)
        )
    except ClientError as e:
        if e.response.get('Error', {}).get('Code') == 'GoneException':
            logger.info(f"Connection {connection_id} is gone")
            # Clean up stale connection
            try:
                connections_table.delete_item(Key={'connectionId': connection_id})
            except Exception as cleanup_error:
                logger.error(f"Error cleaning up connection: {str(cleanup_error)}")
        else:
            logger.error(f"Error sending message: {str(e)}")
      `),
      handler: 'index.handler',
      timeout: cdk.Duration.seconds(30),
      environment: {
        CONNECTIONS_TABLE: this.connectionsTable.tableName,
        STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
        WEBSOCKET_ENDPOINT: this.webSocketStage.url,
      },
      logRetention: logs.RetentionDays.ONE_WEEK,
      loggingFormat: lambda.LoggingFormat.JSON,
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

def deserialize_dynamodb_value(value):
    """Deserialize a single DynamoDB attribute value"""
    if 'S' in value:
        return value['S']
    elif 'N' in value:
        return float(value['N']) if '.' in value['N'] else int(value['N'])
    elif 'BOOL' in value:
        return value['BOOL']
    elif 'M' in value:
        return deserialize_dynamodb_item(value['M'])
    elif 'L' in value:
        return [deserialize_dynamodb_value(item) for item in value['L']]
    elif 'NULL' in value:
        return None
    else:
        return value

def deserialize_dynamodb_item(item):
    if not item:
        return None
    
    # Convert DynamoDB types to Python types
    result = {}
    for key, value in item.items():
        result[key] = deserialize_dynamodb_value(value)
    
    return result
      `),
      handler: 'index.handler',
      timeout: cdk.Duration.seconds(30),
      environment: {
        CONNECTIONS_TABLE: this.connectionsTable.tableName,
        WEBSOCKET_ENDPOINT: this.webSocketStage.url,
      },
      loggingFormat: lambda.LoggingFormat.JSON,
    });

    // Grant permissions to the Lambda functions
    this.connectionsTable.grantReadWriteData(connectHandler);
    this.connectionsTable.grantReadWriteData(disconnectHandler);
    this.connectionsTable.grantReadData(messageHandler);
    props.streamWidgetsTable.grantReadWriteData(messageHandler);
    this.connectionsTable.grantReadData(taskChangeHandler);
    props.streamWidgetsTable.grantReadData(authorizerLambda);

    messageHandler.addToRolePolicy(
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

    // Add default route for widget messages
    this.webSocketApi.addRoute('$default', {
      integration: new WebSocketLambdaIntegration(
        'MessageIntegration',
        messageHandler,
      ),
    });

    // Connect the task change handler to the DynamoDB stream
    taskChangeHandler.addEventSource(
      new DynamoEventSource(props.tasksTable, {
        startingPosition: lambda.StartingPosition.LATEST,
      }),
    );

    // Create Lambda for handling widget changes and broadcasting to WebSocket (Python)
    const widgetChangeHandler = new lambda.Function(this, 'WidgetChangeHandler', {
      runtime: lambda.Runtime.PYTHON_3_11,
      code: lambda.Code.fromInline(`
import json
import os
import boto3
import logging
from botocore.exceptions import ClientError

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

# Initialize API Gateway Management API client
endpoint = os.environ.get('WEBSOCKET_ENDPOINT', '')
if endpoint.startswith('wss://'):
    endpoint = endpoint.replace('wss://', 'https://')
api_client = boto3.client('apigatewaymanagementapi', endpoint_url=endpoint) if endpoint else None

def handler(event, context):
    logger.info(f"Widget change event received")
    
    if not api_client:
        logger.error(f"No valid WebSocket endpoint configured: {os.environ.get('WEBSOCKET_ENDPOINT')}")
        return
    
    # Process each record in the DynamoDB stream
    for record in event.get('Records', []):
        if record.get('eventName') in ['MODIFY', 'INSERT']:
            try:
                # Extract the new and old images
                if 'dynamodb' in record and 'NewImage' in record['dynamodb']:
                    new_widget = deserialize_dynamodb_item(record['dynamodb']['NewImage'])
                    old_widget = deserialize_dynamodb_item(record['dynamodb'].get('OldImage'))
                    handle_widget_change(new_widget, old_widget, record.get('eventName'))
            except Exception as e:
                logger.error(f"Error processing record: {str(e)}")

def handle_widget_change(widget, old_widget, event_name):
    if not widget or 'id' not in widget:
        logger.warning("No valid widget data found in the record")
        return
    
    widget_id = widget.get('id')
    
    # Check if config changed (for WIDGET_CONFIG_UPDATE)
    config_changed = False
    if event_name == 'MODIFY' and old_widget:
        config_changed = widget.get('config') != old_widget.get('config')
    
    # Check if state changed (for WIDGET_STATE_UPDATE)
    state_changed = False
    if event_name == 'MODIFY' and old_widget:
        state_changed = widget.get('state') != old_widget.get('state')
    
    # Find connections subscribed to this widget
    user_id = widget.get('user_id')
    if not user_id:
        logger.warning(f"No user_id found for widget: {widget_id}")
        return
    
    try:
        # Find all connections for this user that are subscribed to this widget
        connections = find_connections_for_widget(user_id, widget_id)
        
        if not connections:
            logger.info(f"No active connections subscribed to widget {widget_id}")
            return
        
        # Broadcast config update if config changed
        if config_changed:
            config_message = json.dumps({
                'type': 'WIDGET_CONFIG_UPDATE',
                'widgetId': widget_id,
                'config': widget.get('config', {})
            })
            broadcast_to_connections(connections, config_message)
        
        # Broadcast state update if state changed
        if state_changed:
            state_message = json.dumps({
                'type': 'WIDGET_STATE_UPDATE',
                'widgetId': widget_id,
                'state': widget.get('state', {}),
                'timestamp': widget.get('updated_at', '')
            })
            broadcast_to_connections(connections, state_message)
    
    except Exception as e:
        logger.error(f"Error processing widget change: {str(e)}")

def find_connections_for_widget(user_id, widget_id):
    """Find all connections for a user that are subscribed to a specific widget"""
    try:
        response = connections_table.query(
            IndexName='user_id-index',
            KeyConditionExpression='user_id = :userId',
            ExpressionAttributeValues={
                ':userId': user_id
            }
        )
        
        # Filter connections that have widget_id in their subscribed_widgets set
        subscribed_connections = []
        for item in response.get('Items', []):
            subscribed_widgets = item.get('subscribed_widgets', set())
            if widget_id in subscribed_widgets:
                subscribed_connections.append(item.get('connectionId'))
        
        return subscribed_connections
    
    except Exception as e:
        logger.error(f"Error querying connections for widget: {str(e)}")
        return []

def broadcast_to_connections(connections, message):
    for connection_id in connections:
        try:
            api_client.post_to_connection(
                ConnectionId=connection_id,
                Data=message
            )
            logger.info(f"Successfully sent update to connection {connection_id}")
        except ClientError as e:
            # Handle stale connections
            if e.response.get('Error', {}).get('Code') == 'GoneException':
                logger.info(f"Connection {connection_id} is stale, removing it")
                remove_connection(connection_id)
            else:
                logger.error(f"Error sending message to connection {connection_id}: {str(e)}")

def remove_connection(connection_id):
    try:
        connections_table.delete_item(
            Key={
                'connectionId': connection_id
            }
        )
    except Exception as e:
        logger.error(f"Error removing stale connection {connection_id}: {str(e)}")

def deserialize_dynamodb_value(value):
    """Deserialize a single DynamoDB attribute value"""
    if 'S' in value:
        return value['S']
    elif 'N' in value:
        return float(value['N']) if '.' in value['N'] else int(value['N'])
    elif 'BOOL' in value:
        return value['BOOL']
    elif 'M' in value:
        return deserialize_dynamodb_item(value['M'])
    elif 'L' in value:
        return [deserialize_dynamodb_value(item) for item in value['L']]
    elif 'NULL' in value:
        return None
    else:
        return value

def deserialize_dynamodb_item(item):
    if not item:
        return None
    
    # Convert DynamoDB types to Python types
    result = {}
    for key, value in item.items():
        result[key] = deserialize_dynamodb_value(value)
    
    return result
      `),
      handler: 'index.handler',
      timeout: cdk.Duration.seconds(30),
      environment: {
        CONNECTIONS_TABLE: this.connectionsTable.tableName,
        WEBSOCKET_ENDPOINT: this.webSocketStage.url,
      },
      logRetention: logs.RetentionDays.ONE_WEEK,
      loggingFormat: lambda.LoggingFormat.JSON,
    });

    // Grant permissions to widget change handler
    this.connectionsTable.grantReadData(widgetChangeHandler);
    widgetChangeHandler.addToRolePolicy(
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

    // Connect the widget change handler to the DynamoDB stream
    widgetChangeHandler.addEventSource(
      new DynamoEventSource(props.streamWidgetsTable, {
        startingPosition: lambda.StartingPosition.LATEST,
      }),
    );
  }
}
