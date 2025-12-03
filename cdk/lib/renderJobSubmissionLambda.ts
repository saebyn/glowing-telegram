import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as lambdaPython from '@aws-cdk/aws-lambda-python-alpha';
import * as logs from 'aws-cdk-lib/aws-logs';
import type * as batch from 'aws-cdk-lib/aws-batch';
import * as iam from 'aws-cdk-lib/aws-iam';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from './util/serviceLambda';

interface RenderJobSubmissionLambdaProps {
  renderJobQueue: batch.IJobQueue;
  renderJobDefinition: batch.IJobDefinition;
}

export default class RenderJobSubmissionLambda extends Construct {
  lambda: lambda.IFunction;

  constructor(
    scope: Construct,
    id: string,
    props: RenderJobSubmissionLambdaProps,
  ) {
    super(scope, id);

    const renderJobSubmissionLogGroup = new logs.LogGroup(
      this,
      'RenderJobSubmissionLogGroup',
      {
        logGroupName: `${LOG_GROUP_PREFIX}/lambda/render-job-submission`,
        retention: LOG_RETENTION,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      },
    );

    const inlineLambda = new lambdaPython.PythonFunction(
      this,
      'RenderJobProcessingLambda',
      {
        runtime: lambda.Runtime.PYTHON_3_13,
        handler: 'handler',
        entry: 'lib/renderJobSubmissionLambda',
        index: 'handler.py',
        tracing: lambda.Tracing.ACTIVE,
        loggingFormat: lambda.LoggingFormat.JSON,
        logGroup: renderJobSubmissionLogGroup,
        environment: {
          RENDER_JOB_QUEUE: props.renderJobQueue.jobQueueArn,
          RENDER_JOB_DEFINITION: props.renderJobDefinition.jobDefinitionArn,
          MAX_EPISODES_PER_JOB: '3',
        },
      },
    );

    inlineLambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['batch:SubmitJob'],
        resources: [
          props.renderJobQueue.jobQueueArn,
          props.renderJobDefinition.jobDefinitionArn,
        ],
      }),
    );

    this.lambda = inlineLambda;
  }
}
