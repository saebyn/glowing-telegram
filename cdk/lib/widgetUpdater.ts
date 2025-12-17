import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as events from 'aws-cdk-lib/aws-events';
import * as targets from 'aws-cdk-lib/aws-events-targets';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as iam from 'aws-cdk-lib/aws-iam';
import ServiceLambdaConstruct from './util/serviceLambda';

export interface WidgetUpdaterConstructProps {
  streamWidgetsTable: dynamodb.ITable;
  tagOrDigest?: string;
}

export default class WidgetUpdaterConstruct extends Construct {
  constructor(scope: Construct, id: string, props: WidgetUpdaterConstructProps) {
    super(scope, id);

    const service = new ServiceLambdaConstruct(this, 'WidgetUpdaterService', {
      name: 'widget-updater',
      tagOrDigest: props.tagOrDigest,
      lambdaOptions: {
        timeout: cdk.Duration.seconds(1),
        memorySize: 256,
        environment: {
          STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
        },
        description: 'Processes scheduled updates for stream widgets',
      },
    });

    // Grant read/write access to stream_widgets table
    props.streamWidgetsTable.grantReadWriteData(service.lambda);

    // Create EventBridge rule for countdown widgets (every 1 second)
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

    // Future widget types can be added here with different schedules
    // Example: Poll widgets every 5 minutes
    // const pollRule = new events.Rule(this, 'PollWidgetUpdateRule', {
    //   schedule: events.Schedule.rate(cdk.Duration.minutes(5)),
    //   description: 'Triggers widget updater for poll widgets',
    // });
    //
    // pollRule.addTarget(
    //   new targets.LambdaFunction(this.widgetUpdaterFunction, {
    //     event: events.RuleTargetInput.fromObject({
    //       widget_type: 'poll',
    //     }),
    //   }),
    // );
  }
}
