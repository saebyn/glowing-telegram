import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as lambdaPython from '@aws-cdk/aws-lambda-python-alpha';
import type * as batch from 'aws-cdk-lib/aws-batch';
import * as iam from 'aws-cdk-lib/aws-iam';

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

    const inlineLambda = new lambdaPython.PythonFunction(
      this,
      'RenderJobProcessingLambda',
      {
        runtime: lambda.Runtime.PYTHON_3_11,
        handler: 'handler',
        entry: 'lib/renderJobSubmissionLambda',
        index: 'handler.py',
        tracing: lambda.Tracing.ACTIVE,
        loggingFormat: lambda.LoggingFormat.JSON,
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
