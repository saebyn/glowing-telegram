import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as events from 'aws-cdk-lib/aws-events';
import * as event_targets from 'aws-cdk-lib/aws-events-targets';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as iam from 'aws-cdk-lib/aws-iam';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from './util/serviceLambda';

interface TaskMonitoringConstructProps {
  tasksTable: dynamodb.ITable;
}

export default class TaskMonitoringConstruct extends Construct {
  private statusLambda: lambda.IFunction;

  constructor(
    scope: Construct,
    id: string,
    props: TaskMonitoringConstructProps,
  ) {
    super(scope, id);

    const { tasksTable } = props;

    const statusLogGroup = new logs.LogGroup(this, 'StatusLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/lambda/task-status`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    const statusLambda = new lambda.Function(this, 'StatusLambda', {
      runtime: lambda.Runtime.PYTHON_3_13,
      code: lambda.Code.fromInline(`
import json
import os
import boto3
from datetime import datetime, timedelta, timezone

def handler(event, context):
    dynamodb = boto3.resource('dynamodb')

    now = datetime.now(timezone.utc).isoformat()

    ttl = datetime.now() + timedelta(days=7)
    ttl_value = int(ttl.timestamp())

    table = dynamodb.Table(os.environ['TASKS_TABLE_NAME'])

    record_id = event.get('record_id', None)
    task_type = event.get('task_type', None)

    expression_attribute_names = {
        '#status': 'status',
        '#time': 'time',
        '#createdAt': 'created_at',
        '#updatedAt': 'updated_at',
        '#userId': 'user_id',
        '#ttl': 'ttl',
    }
    expression_attribute_values = {
        ':status': event['status'],
        ':time': event['time'],
        ':userId': event['user_id'],
        ':now': now,
        ':ttl': ttl_value,
    }
    sets_parts = [
        '#status = :status',
        '#time = :time',
        '#userId = :userId',
        '#createdAt = if_not_exists(#createdAt, :now)',
        '#updatedAt = :now',
        '#ttl = :ttl',
    ]

    if record_id is not None:
        sets_parts.append('#record_id = if_not_exists(#record_id, :record_id)')
        expression_attribute_names['#record_id'] = 'record_id'
        expression_attribute_values[':record_id'] = record_id

    if task_type is not None:
        sets_parts.append('#task_type = if_not_exists(#task_type, :task_type)')
        expression_attribute_names['#task_type'] = 'task_type'
        expression_attribute_values[':task_type'] = task_type

    update_expression = 'SET ' + ', '.join(sets_parts)

    table.update_item(
        Key={'id': event['name']},
        UpdateExpression=update_expression,
        ExpressionAttributeNames=expression_attribute_names,
        ExpressionAttributeValues=expression_attribute_values,
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
      tracing: lambda.Tracing.ACTIVE,
      loggingFormat: lambda.LoggingFormat.JSON,
      logGroup: statusLogGroup,
      initialPolicy: [
        new iam.PolicyStatement({
          actions: ['dynamodb:UpdateItem'],
          resources: [tasksTable.tableArn],
        }),
      ],
    });

    this.statusLambda = statusLambda;
  }

  public newEventTarget({
    name,
    status,
    time,
    user_id,
    record_id,
    task_type,
  }: {
    name: string;
    status: string;
    time: string;
    user_id: string;
    record_id?: string;
    task_type?: string;
  }): events.IRuleTarget {
    return new event_targets.LambdaFunction(this.statusLambda, {
      event: events.RuleTargetInput.fromObject({
        name,
        status,
        time,
        user_id,
        record_id: record_id || undefined,
        task_type: task_type || undefined,
      }),
    });
  }
}
