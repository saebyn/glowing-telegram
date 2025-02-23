import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as apigwv2 from 'aws-cdk-lib/aws-apigatewayv2';
import type * as cognito from 'aws-cdk-lib/aws-cognito';
import { HttpUserPoolAuthorizer } from 'aws-cdk-lib/aws-apigatewayv2-authorizers';
import {
  HttpStepFunctionsIntegration,
  HttpLambdaIntegration,
} from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import type { StateMachine } from 'aws-cdk-lib/aws-stepfunctions';
import * as events from 'aws-cdk-lib/aws-events';
import * as eventTargets from 'aws-cdk-lib/aws-events-targets';
import * as iam from 'aws-cdk-lib/aws-iam';
import type * as batch from 'aws-cdk-lib/aws-batch';
import type { ITable } from 'aws-cdk-lib/aws-dynamodb';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import ServiceLambdaConstruct from './util/serviceLambda';
import RenderJobSubmissionLambda from './renderJobSubmissionLambda';

interface APIConstructProps {
  userPool: cognito.IUserPool;
  userPoolClients: cognito.IUserPoolClient[];
  streamIngestionFunction: StateMachine;
  videoMetadataTable: ITable;
  streamsTable: ITable;
  streamSeriesTable: ITable;
  episodesTable: ITable;
  profilesTable: ITable;
  tasksTable: ITable;
  openaiSecret: secretsmanager.ISecret;

  domainName: string;

  renderJob: {
    jobQueue: batch.IJobQueue;
    jobDefinition: batch.IJobDefinition;
  };
}

export default class APIConstruct extends Construct {
  public readonly httpApi: apigwv2.HttpApi;

  constructor(scope: Construct, id: string, props: APIConstructProps) {
    super(scope, id);

    // youtube lambda
    const youtubeAppSecret = new secretsmanager.Secret(
      this,
      'YoutubeAppSecret',
      {
        description: 'Youtube App Secret for API access in glowing-telegram',
        removalPolicy: cdk.RemovalPolicy.RETAIN,
      },
    );

    const youtubeService = new ServiceLambdaConstruct(this, 'YoutubeLambda', {
      lambdaOptions: {
        description: 'Youtube OAuth Lambda for Glowing-Telegram',
        timeout: cdk.Duration.seconds(30),
        environment: {
          USER_SECRET_PATH: 'gt/youtube/user',
          YOUTUBE_SECRET_ARN: youtubeAppSecret.secretArn,
        },
      },
      name: 'youtube-lambda',
    });

    youtubeAppSecret.grantRead(youtubeService.lambda);
    youtubeService.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          // put, create, get secret values
          'secretsmanager:GetSecretValue',
          'secretsmanager:PutSecretValue',
          'secretsmanager:CreateSecret',
        ],
        resources: [
          cdk.Arn.format(
            {
              service: 'secretsmanager',
              resource: 'secret',
              resourceName: 'gt/youtube/user/*',
              arnFormat: cdk.ArnFormat.COLON_RESOURCE_NAME,
            },
            cdk.Stack.of(this),
          ),
        ],
      }),
    );

    // twitch lambda
    const twitchAppSecret = new secretsmanager.Secret(this, 'TwitchAppSecret', {
      description: 'Twitch App Secret for API access in glowing-telegram',
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const twitchService = new ServiceLambdaConstruct(this, 'TwitchLambda', {
      lambdaOptions: {
        description: 'Twitch OAuth Lambda for Glowing-Telegram',
        timeout: cdk.Duration.seconds(30),
        environment: {
          USER_SECRET_PATH: 'gt/twitch/user',
          TWITCH_SECRET_ARN: twitchAppSecret.secretArn,
          IS_GLOBAL_REFRESH_SERVICE: 'false',
        },
      },
      name: 'twitch-lambda',
    });

    twitchAppSecret.grantRead(twitchService.lambda);
    twitchService.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          // put, create, get secret values
          'secretsmanager:GetSecretValue',
          'secretsmanager:PutSecretValue',
          'secretsmanager:CreateSecret',
        ],
        resources: [
          cdk.Arn.format(
            {
              service: 'secretsmanager',
              resource: 'secret',
              resourceName: 'gt/twitch/user/*',
              arnFormat: cdk.ArnFormat.COLON_RESOURCE_NAME,
            },
            cdk.Stack.of(this),
          ),
        ],
      }),
    );

    // create a lambda and an event rule to run it every hour to refresh the twitch tokens for all users
    const tokenRefreshLambda = new ServiceLambdaConstruct(
      this,
      'TokenRefreshLambda',
      {
        name: 'twitch-lambda',
        lambdaOptions: {
          description: 'Twitch Token Refresh Lambda for Glowing-Telegram',
          timeout: cdk.Duration.minutes(5),
          environment: {
            USER_SECRET_PATH: 'gt/twitch/user',
            TWITCH_SECRET_ARN: twitchAppSecret.secretArn,
            IS_GLOBAL_REFRESH_SERVICE: 'true',
          },
        },
      },
    );

    // grant the lambda permissions to read the secret
    twitchAppSecret.grantRead(tokenRefreshLambda.lambda);
    tokenRefreshLambda.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          'secretsmanager:GetSecretValue',
          'secretsmanager:PutSecretValue',
        ],
        resources: [
          cdk.Arn.format(
            {
              service: 'secretsmanager',
              resource: 'secret',
              resourceName: 'gt/twitch/user/*',
              arnFormat: cdk.ArnFormat.COLON_RESOURCE_NAME,
            },
            cdk.Stack.of(this),
          ),
        ],
      }),
    );
    tokenRefreshLambda.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['secretsmanager:ListSecrets'],
        resources: ['*'],
      }),
    );

    // configure the event rule to run every hour
    new events.Rule(this, 'TokenRefreshRule', {
      schedule: events.Schedule.rate(cdk.Duration.hours(1)),
      enabled: true,
    }).addTarget(new eventTargets.LambdaFunction(tokenRefreshLambda.lambda));

    // configure crud lambda
    const crudService = new ServiceLambdaConstruct(this, 'CrudLambda', {
      lambdaOptions: {
        description: 'CRUD operations for the Glowing-Telegram API',
        timeout: cdk.Duration.seconds(30),
        environment: {
          VIDEO_METADATA_TABLE: props.videoMetadataTable.tableName,
          STREAMS_TABLE: props.streamsTable.tableName,
          SERIES_TABLE: props.streamSeriesTable.tableName,
          EPISODES_TABLE: props.episodesTable.tableName,
          PROFILES_TABLE: props.profilesTable.tableName,
          TASKS_TABLE: props.tasksTable.tableName,
        },
      },
      name: 'crud-lambda',
    });

    crudService.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          'dynamodb:BatchGetItem',
          'dynamodb:BatchWriteItem',
          'dynamodb:DeleteItem',
          'dynamodb:GetItem',
          'dynamodb:PutItem',
          'dynamodb:Query',
          'dynamodb:Scan',
          'dynamodb:UpdateItem',
        ],
        resources: [
          props.videoMetadataTable.tableArn,
          props.streamsTable.tableArn,
          props.streamSeriesTable.tableArn,
          props.episodesTable.tableArn,
          props.profilesTable.tableArn,
          props.tasksTable.tableArn,

          // Allow access to the indexes
          `${props.videoMetadataTable.tableArn}/index/*`,
          `${props.streamsTable.tableArn}/index/*`,
          `${props.streamSeriesTable.tableArn}/index/*`,
          `${props.episodesTable.tableArn}/index/*`,
          `${props.profilesTable.tableArn}/index/*`,
          `${props.tasksTable.tableArn}/index/*`,
        ],
      }),
    );

    // configure ai chat lambda
    const aiChatService = new ServiceLambdaConstruct(this, 'AiChatLambda', {
      lambdaOptions: {
        description: 'AI Chat Lambda for Glowing-Telegram',
        timeout: cdk.Duration.minutes(3),
        environment: {
          OPENAI_SECRET_ARN: props.openaiSecret.secretArn,
          OPENAI_MODEL: 'gpt-4o-2024-11-20',
        },
      },
      name: 'ai-chat-lambda',
    });

    props.openaiSecret.grantRead(aiChatService.lambda);

    // configure authorizer
    const authorizer = new HttpUserPoolAuthorizer(
      'Authorizer',
      props.userPool,
      {
        userPoolClients: props.userPoolClients,
      },
    );

    const httpApi = new apigwv2.HttpApi(this, 'HttpApi', {
      defaultAuthorizer: authorizer,

      corsPreflight: {
        allowOrigins: ['http://localhost:5173', `https://${props.domainName}`],
        allowMethods: [apigwv2.CorsHttpMethod.ANY],
        allowHeaders: ['authorization', 'content-type', 'accept'],
        exposeHeaders: [
          'location',
          'content-range',
          'content-length',
          'content-type',
        ],
        allowCredentials: true,
        maxAge: cdk.Duration.days(1),
      },
    });

    this.httpApi = httpApi;

    const renderJobSubmissionLambda = new RenderJobSubmissionLambda(
      this,
      'RenderJobSubmissionLambda',
      {
        renderJobQueue: props.renderJob.jobQueue,
        renderJobDefinition: props.renderJob.jobDefinition,
      },
    );

    // configure routes

    // POST /stream - run stream ingestion step function
    httpApi.addRoutes({
      integration: new HttpStepFunctionsIntegration(
        'StreamIngestionIntegration',
        {
          stateMachine: props.streamIngestionFunction,
          parameterMapping: new apigwv2.ParameterMapping()
            .custom('Input', '$request.body')
            .custom(
              'StateMachineArn',
              props.streamIngestionFunction.stateMachineArn,
            ),

          subtype: apigwv2.HttpIntegrationSubtype.STEPFUNCTIONS_START_EXECUTION,
        },
      ),
      path: '/stream',
      methods: [apigwv2.HttpMethod.POST],
    });

    // ANY /crud - run crud lambda
    httpApi.addRoutes({
      integration: new HttpLambdaIntegration(
        'CrudIntegration',
        crudService.lambda,
      ),
      path: '/records/{proxy+}',
      methods: [
        apigwv2.HttpMethod.DELETE,
        apigwv2.HttpMethod.GET,
        apigwv2.HttpMethod.PATCH,
        apigwv2.HttpMethod.POST,
        apigwv2.HttpMethod.PUT,
      ],
    });

    // POST/GET /auth/twitch/* - run twitch lambda
    httpApi.addRoutes({
      integration: new HttpLambdaIntegration(
        'TwitchIntegration',
        twitchService.lambda,
      ),
      path: '/auth/twitch/{proxy+}',
      methods: [apigwv2.HttpMethod.POST, apigwv2.HttpMethod.GET],
    });

    // POST/GET /auth/youtube/* - run youtube lambda
    httpApi.addRoutes({
      integration: new HttpLambdaIntegration(
        'YoutubeIntegration',
        youtubeService.lambda,
      ),

      path: '/auth/youtube/{proxy+}', // specify path for youtube integration
      methods: [apigwv2.HttpMethod.POST, apigwv2.HttpMethod.GET], // allow POST and GET methods
    });

    // POST /ai/chat - run ai chat lambda
    httpApi.addRoutes({
      integration: new HttpLambdaIntegration(
        'AiChatIntegration',
        aiChatService.lambda,
      ),
      path: '/ai/chat',
      methods: [apigwv2.HttpMethod.POST],
    });

    // POST /render - trigger the render job
    httpApi.addRoutes({
      integration: new HttpLambdaIntegration(
        'TrackCutListProcessingIntegration',
        renderJobSubmissionLambda.lambda,
      ),
      path: '/render',
      methods: [apigwv2.HttpMethod.POST],
    });
  }
}
