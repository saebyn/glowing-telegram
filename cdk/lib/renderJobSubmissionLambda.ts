import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
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

    const inlineLambda = new lambda.Function(
      this,
      'RenderJobProcessingLambda',
      {
        runtime: lambda.Runtime.PYTHON_3_9,
        handler: 'index.handler',
        code: lambda.Code.fromInline(`
import json
import boto3
import os
import datetime
import hashlib

batch = boto3.client('batch')

# This lambda function is triggered by an API Gateway v2 HTTP API endpoint
def handler(event, context):
    job_queue_arn = os.environ['RENDER_JOB_QUEUE']
    job_definition_arn = os.environ['RENDER_JOB_DEFINITION']

    request_body = json.loads(event['body'])
    episode_ids = request_body['episodeIds']

    job_name = hashlib.md5(''.join(episode_ids).encode('utf-8')).hexdigest()

    result = batch.submit_job(
        jobName=f'cut-list-render-job-{job_name}',
        jobQueue=job_queue_arn,
        jobDefinition=job_definition_arn,
        parameters={'record_ids': ' '.join(episode_ids)}
    )

    response = {
        'message': 'Job submitted successfully',
        'jobId': result['jobId']
    }


    return {'statusCode': 200, 'body': json.dumps(response)}
      `),
        environment: {
          RENDER_JOB_QUEUE: props.renderJobQueue.jobQueueArn,
          RENDER_JOB_DEFINITION: props.renderJobDefinition.jobDefinitionArn,
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
