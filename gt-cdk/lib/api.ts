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
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as iam from 'aws-cdk-lib/aws-iam';
import type { ITable } from 'aws-cdk-lib/aws-dynamodb';
import type { ISecret } from 'aws-cdk-lib/aws-secretsmanager';
import ServiceLambdaConstruct from './util/serviceLambda';

interface APIConstructProps {
  userPool: cognito.IUserPool;
  userPoolClients: cognito.IUserPoolClient[];
  streamIngestionFunction: StateMachine;
  videoMetadataTable: ITable;
  streamsTable: ITable;
  streamSeriesTable: ITable;
  episodesTable: ITable;
  profilesTable: ITable;
  openaiSecret: ISecret;
}

export default class APIConstruct extends Construct {
  constructor(scope: Construct, id: string, props: APIConstructProps) {
    super(scope, id);

    // configure crud lambda
    const crudService = new ServiceLambdaConstruct(this, 'CrudLambda', {
      lambdaOptions: {
        timeout: cdk.Duration.seconds(30),
        environment: {
          VIDEO_METADATA_TABLE: props.videoMetadataTable.tableName,
          STREAMS_TABLE: props.streamsTable.tableName,
          STREAM_SERIES_TABLE: props.streamSeriesTable.tableName,
          EPISODES_TABLE: props.episodesTable.tableName,
          PROFILES_TABLE: props.profilesTable.tableName,
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

          // Allow access to the indexes
          `${props.videoMetadataTable.tableArn}/index/*`,
          `${props.streamsTable.tableArn}/index/*`,
          `${props.streamSeriesTable.tableArn}/index/*`,
          `${props.episodesTable.tableArn}/index/*`,
          `${props.profilesTable.tableArn}/index/*`,
        ],
      }),
    );

    // configure ai chat lambda
    const aiChatService = new ServiceLambdaConstruct(this, 'AiChatLambda', {
      lambdaOptions: {
        timeout: cdk.Duration.minutes(3),
        environment: {
          OPENAI_SECRET: props.openaiSecret.secretArn,
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
      apiName: 'gt-api',

      corsPreflight: {
        allowOrigins: ['http://localhost:5173'],
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

    // POST /ai/chat - run ai chat lambda
    httpApi.addRoutes({
      integration: new HttpLambdaIntegration(
        'AiChatIntegration',
        aiChatService.lambda,
      ),
      path: '/ai/chat',
      methods: [apigwv2.HttpMethod.POST],
    });
  }
}
