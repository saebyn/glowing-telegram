import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as events from 'aws-cdk-lib/aws-events';
import * as targets from 'aws-cdk-lib/aws-events-targets';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import ServiceLambdaConstruct from './util/serviceLambda';

export interface WidgetUpdaterConstructProps {
  streamWidgetsTable: dynamodb.ITable;
  /** The Twitch app secret (contains client_id / client_secret). */
  twitchAppSecret: secretsmanager.ISecret;
  /** Base path for per-user Twitch secrets, e.g. "gt/twitch/user". */
  twitchUserSecretBasePath: string;
  tagOrDigest?: string;
}

export default class WidgetUpdaterConstruct extends Construct {
  constructor(scope: Construct, id: string, props: WidgetUpdaterConstructProps) {
    super(scope, id);

    const service = new ServiceLambdaConstruct(this, 'WidgetUpdaterService', {
      name: 'widget-updater-lambda',
      tagOrDigest: props.tagOrDigest,
      lambdaOptions: {
        timeout: cdk.Duration.seconds(30),
        memorySize: 256,
        environment: {
          STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
          TWITCH_SECRET_ARN: props.twitchAppSecret.secretArn,
          USER_SECRET_PATH: props.twitchUserSecretBasePath,
        },
        description: 'Processes scheduled updates for stream widgets',
      },
    });

    // Grant read/write access to stream_widgets table
    props.streamWidgetsTable.grantReadWriteData(service.lambda);

    // Grant read access to the Twitch app secret (for client_id at cold-start)
    props.twitchAppSecret.grantRead(service.lambda);

    // Grant read access to per-user Twitch secrets (for access tokens)
    service.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['secretsmanager:GetSecretValue'],
        resources: [
          cdk.Arn.format(
            {
              service: 'secretsmanager',
              resource: 'secret',
              resourceName: `${props.twitchUserSecretBasePath}/*`,
              arnFormat: cdk.ArnFormat.COLON_RESOURCE_NAME,
            },
            cdk.Stack.of(this),
          ),
        ],
      }),
    );

    // Create EventBridge rule for countdown widgets (every 1 minute)
    const countdownRule = new events.Rule(this, 'CountdownWidgetUpdateRule', {
      schedule: events.Schedule.rate(cdk.Duration.minutes(1)),
      description: 'Triggers widget updater for countdown widgets',
    });

    countdownRule.addTarget(
      new targets.LambdaFunction(service.lambda, {
        event: events.RuleTargetInput.fromObject({
          widget_type: 'countdown',
        }),
      }),
    );

    // Create EventBridge rule for ad_timer widgets (every 5 minutes).
    // The backend polls the Twitch ad schedule API and pushes state via WebSocket.
    // See src/widgets/ad-timer/BACKEND_INTEGRATION.md in glowing-telegram-frontend.
    const adTimerRule = new events.Rule(this, 'AdTimerWidgetUpdateRule', {
      schedule: events.Schedule.rate(cdk.Duration.minutes(5)),
      description: 'Triggers widget updater for ad_timer widgets (Twitch ad schedule polling)',
    });

    adTimerRule.addTarget(
      new targets.LambdaFunction(service.lambda, {
        event: events.RuleTargetInput.fromObject({
          widget_type: 'ad_timer',
        }),
      }),
    );
  }
}
