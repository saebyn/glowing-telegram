import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import type * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import type * as stepfunctions from 'aws-cdk-lib/aws-stepfunctions';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as tasks from 'aws-cdk-lib/aws-stepfunctions-tasks';
import * as events from 'aws-cdk-lib/aws-events';
import * as event_targets from 'aws-cdk-lib/aws-events-targets';

interface TaskMonitoringConstructProps {
  tasksTable: dynamodb.ITable;
  streamIngestionStepFunction: stepfunctions.IStateMachine;
}

export default class TaskMonitoringConstruct extends Construct {
  constructor(
    scope: Construct,
    id: string,
    props: TaskMonitoringConstructProps,
  ) {
    super(scope, id);

    const { tasksTable, streamIngestionStepFunction } = props;

    const statusLambda = new lambda.Function(this, 'StatusLambda', {
      runtime: lambda.Runtime.PYTHON_3_13,
      code: lambda.Code.fromInline(`
import json
import os
import boto3
from datetime import datetime, timedelta

def handler(event, context):
    dynamodb = boto3.resource('dynamodb')

    now = datetime.now().isoformat()

    ttl = datetime.now() + timedelta(days=7)
    ttl_value = int(ttl.timestamp())

    table = dynamodb.Table(os.environ['TASKS_TABLE_NAME'])
    table.update_item(
        Key={'id': event['name']},
        UpdateExpression='SET #status = :status, #time = :time, #createdAt = if_not_exists(#createdAt, :now), #updatedAt = :now, #ttl = :ttl',
        ConditionExpression='#status = :running OR attribute_not_exists(#status)',
        ExpressionAttributeNames={'#status': 'status', '#time': 'time', '#createdAt': 'created_at', '#updatedAt': 'updated_at', '#ttl': 'ttl'},
        ExpressionAttributeValues={':status': event['status'], ':time': event['time'], ':running': 'RUNNING', ':now': now, ':ttl': ttl_value},
    )
    return {
        'statusCode': 200,
        'body': json.dumps('Success'),
    }
`),
      handler: 'index.handler',
      environment: {
        TASKS_TABLE_NAME: tasksTable.tableName,
      },
      initialPolicy: [
        new iam.PolicyStatement({
          actions: ['dynamodb:UpdateItem'],
          resources: [tasksTable.tableArn],
        }),
      ],
    });

    const stepFunctionStatusRule = new events.Rule(
      this,
      'StepFunctionStatusRule',
      {
        eventPattern: {
          source: ['aws.states'],
          detailType: ['Step Functions Execution Status Change'],
          detail: {
            stateMachineArn: [streamIngestionStepFunction.stateMachineArn],
          },
        },
        enabled: true,
      },
    );
    stepFunctionStatusRule.addTarget(
      new event_targets.LambdaFunction(statusLambda, {
        event: events.RuleTargetInput.fromObject({
          name: events.EventField.fromPath('$.detail.name'),
          status: events.EventField.fromPath('$.detail.status'),
          time: events.EventField.fromPath('$.time'),
        }),
      }),
    );
  }
}
