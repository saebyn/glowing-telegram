import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as events from 'aws-cdk-lib/aws-events';
import * as targets from 'aws-cdk-lib/aws-events-targets';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as iam from 'aws-cdk-lib/aws-iam';

export interface WidgetUpdaterConstructProps {
  streamWidgetsTable: dynamodb.ITable;
}

export default class WidgetUpdaterConstruct extends Construct {
  public readonly widgetUpdaterFunction: lambda.Function;

  constructor(scope: Construct, id: string, props: WidgetUpdaterConstructProps) {
    super(scope, id);

    // Reference the ECR repository for widget updater
    const repository = ecr.Repository.fromRepositoryName(
      this,
      'WidgetUpdaterRepository',
      'glowing-telegram/widget-updater-lambda',
    );

    // Create the widget updater Lambda function
    this.widgetUpdaterFunction = new lambda.DockerImageFunction(
      this,
      'WidgetUpdaterFunction',
      {
        code: lambda.DockerImageCode.fromEcr(repository, {
          tagOrDigest: 'latest',
        }),
        timeout: cdk.Duration.seconds(30),
        memorySize: 256,
        architecture: lambda.Architecture.ARM_64,
        environment: {
          STREAM_WIDGETS_TABLE: props.streamWidgetsTable.tableName,
        },
        description: 'Processes scheduled updates for stream widgets',
      },
    );

    // Grant read/write access to stream_widgets table
    props.streamWidgetsTable.grantReadWriteData(this.widgetUpdaterFunction);

    // Grant access to the GSI for querying by type
    this.widgetUpdaterFunction.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['dynamodb:Query'],
        resources: [`${props.streamWidgetsTable.tableArn}/index/*`],
      }),
    );

    // Create EventBridge rule for countdown widgets (every 1 second)
    const countdownRule = new events.Rule(this, 'CountdownWidgetUpdateRule', {
      schedule: events.Schedule.rate(cdk.Duration.seconds(1)),
      description: 'Triggers widget updater for countdown widgets every second',
    });

    countdownRule.addTarget(
      new targets.LambdaFunction(this.widgetUpdaterFunction, {
        event: events.RuleTargetInput.fromObject({
          widget_type: 'countdown',
        }),
      }),
    );

    // Future widget types can be added here with different schedules
    // Example: Poll widgets every 5 seconds
    // const pollRule = new events.Rule(this, 'PollWidgetUpdateRule', {
    //   schedule: events.Schedule.rate(cdk.Duration.seconds(5)),
    //   description: 'Triggers widget updater for poll widgets every 5 seconds',
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
