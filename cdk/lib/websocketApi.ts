import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as apigwv2 from 'aws-cdk-lib/aws-apigatewayv2';
import type * as cognito from 'aws-cdk-lib/aws-cognito';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import { WebSocketLambdaIntegration } from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import { WebSocketLambdaAuthorizer } from 'aws-cdk-lib/aws-apigatewayv2-authorizers';
import { DynamoEventSource } from 'aws-cdk-lib/aws-lambda-event-sources';
import ServiceLambdaConstruct from './util/serviceLambda';

interface WebSocketAPIConstructProps {
  tagOrDigest?: string;
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

    // Add widgetId index to look up connections by widget (for WidgetAccess connections)
    // WidgetAccess connections are authenticated with a widget token and automatically
    // receive updates for that widget without explicit subscription
    this.connectionsTable.addGlobalSecondaryIndex({
      indexName: 'widgetId-index',
      partitionKey: { name: 'widgetId', type: dynamodb.AttributeType.STRING },
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

    // Create the authorizer Lambda for WebSocket API (Docker)
    const authorizerService = new ServiceLambdaConstruct(
      this,
      'AuthorizerLambda',
      {
        name: 'websocket-lambda',
        tagOrDigest: props.tagOrDigest,
        lambdaOptions: {
          description: 'WebSocket Authorizer Lambda',
          timeout: cdk.Duration.seconds(10),
          handler: 'authorizer.handler',
          environment: {
            USER_POOL_ID: props.userPool.userPoolId,
            USER_POOL_CLIENT_ID: props.userPoolClient.userPoolClientId,
            STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
          },
        },
      },
    );
    const authorizerLambda = authorizerService.lambda;

    // Create Lambda for handling WebSocket connect events (Docker)
    const connectService = new ServiceLambdaConstruct(
      this,
      'ConnectHandler',
      {
        name: 'websocket-lambda',
        tagOrDigest: props.tagOrDigest,
        lambdaOptions: {
          description: 'WebSocket Connect Handler',
          timeout: cdk.Duration.seconds(30),
          handler: 'connect.handler',
          environment: {
            CONNECTIONS_TABLE: this.connectionsTable.tableName,
          },
        },
      },
    );
    const connectHandler = connectService.lambda;

    // Create Lambda for handling WebSocket disconnect events (Docker)
    const disconnectService = new ServiceLambdaConstruct(
      this,
      'DisconnectHandler',
      {
        name: 'websocket-lambda',
        tagOrDigest: props.tagOrDigest,
        lambdaOptions: {
          description: 'WebSocket Disconnect Handler',
          timeout: cdk.Duration.seconds(30),
          handler: 'disconnect.handler',
          environment: {
            CONNECTIONS_TABLE: this.connectionsTable.tableName,
          },
        },
      },
    );
    const disconnectHandler = disconnectService.lambda;

    // Create Lambda for handling WebSocket messages (Python)
    const messageService = new ServiceLambdaConstruct(
      this,
      'MessageHandler',
      {
        name: 'websocket-lambda',
        tagOrDigest: props.tagOrDigest,
        lambdaOptions: {
          description: 'WebSocket Message Handler',
          timeout: cdk.Duration.seconds(30),
          handler: 'message.handler',
          environment: {
            CONNECTIONS_TABLE: this.connectionsTable.tableName,
            STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
            WEBSOCKET_ENDPOINT: this.webSocketStage.url
          },
        },
      },
    );
    const messageHandler = messageService.lambda;

    // Create Lambda for handling task changes and publishing to WebSocket (Python)
    const taskChangeService = new ServiceLambdaConstruct(
      this,
      'TaskChangeHandler',
      {
        name: 'websocket-lambda',
        tagOrDigest: props.tagOrDigest,
        lambdaOptions: {
          description: 'WebSocket Task Change Handler',
          timeout: cdk.Duration.seconds(30),
          handler: 'task_change.handler',
          environment: {
            CONNECTIONS_TABLE: this.connectionsTable.tableName,
            WEBSOCKET_ENDPOINT: this.webSocketStage.url
          },
        },
      },
    );
    const taskChangeHandler = taskChangeService.lambda;

    // Grant permissions to the Lambda functions
    this.connectionsTable.grantReadWriteData(connectHandler);
    this.connectionsTable.grantReadWriteData(disconnectHandler);
    this.connectionsTable.grantReadWriteData(messageHandler); // Write needed for subscription updates
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
    const widgetChangeService = new ServiceLambdaConstruct(
      this,
      'WidgetChangeHandler',
      {
        name: 'websocket-lambda',
        tagOrDigest: props.tagOrDigest,
        lambdaOptions: {
          description: 'WebSocket Widget Change Handler',
          timeout: cdk.Duration.seconds(30),
          handler: 'widget_change.handler',
          environment: {
            CONNECTIONS_TABLE: this.connectionsTable.tableName,
            WEBSOCKET_ENDPOINT: this.webSocketStage.url,
          },
        },
      },
    );
    const widgetChangeHandler = widgetChangeService.lambda;

    // Grant permissions to widget change handler
    this.connectionsTable.grantReadWriteData(widgetChangeHandler); // Write needed to delete stale connections
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
