import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as batch from 'aws-cdk-lib/aws-batch';
import * as iam from 'aws-cdk-lib/aws-iam';

interface TrackCutListProcessingLambdaProps {
  episodesTable: dynamodb.ITable;
  renderJobQueue: batch.IJobQueue;
  renderJobDefinition: batch.IJobDefinition;
}

export default class TrackCutListProcessingLambda extends Construct {
  lambda: lambda.IFunction;

  constructor(scope: Construct, id: string, props: TrackCutListProcessingLambdaProps) {
    super(scope, id);

    const inlineLambda = new lambda.Function(this, 'InlinePythonLambda', {
      runtime: lambda.Runtime.PYTHON_3_9,
      handler: 'index.handler',
      code: lambda.Code.fromInline(`
import json
import boto3
import os
import datetime

dynamodb = boto3.resource('dynamodb')
batch = boto3.client('batch')

def handler(event, context):
    table_name = os.environ['EPISODES_TABLE']
    job_queue_arn = os.environ['RENDER_JOB_QUEUE']
    job_definition_arn = os.environ['RENDER_JOB_DEFINITION']

    episodes_table = dynamodb.Table(table_name)
    episode_ids = event['episode_ids']

    # TODO batch get items
    # TODO skip if cut_list already exists (use a flag in event)

    for episode_id in episode_ids:
        response = episodes_table.get_item(Key={'id': episode_id})
        episode = response['Item']
        tracks = episode.get('tracks', [])
        cut_list = convert_tracks_to_cut_list(tracks)
        episode['cut_list'] = cut_list
        episodes_table.put_item(Item=episode)

    batch.submit_job(
        jobName=f'cut-list-render-job-{datetime.datetime.now().isoformat()}',
        jobQueue=job_queue_arn,
        jobDefinition=job_definition_arn,
        parameters={'record_ids': ' '.join(episode_ids)}
    )

    return {'statusCode': 200, 'body': json.dumps('Job submitted successfully')}


def convert_tracks_to_cut_list(tracks):
    # TODO Implement the logic to convert tracks to cut list
    cut_list = []
    for track in tracks:
        cut_list.append({'start': track['start'], 'end': track['end']})
    return cut_list
      `),
      environment: {
        EPISODES_TABLE: props.episodesTable.tableName,
        RENDER_JOB_QUEUE: props.renderJobQueue.jobQueueArn,
        RENDER_JOB_DEFINITION: props.renderJobDefinition.jobDefinitionArn,
      },
    });

    props.episodesTable.grantReadWriteData(inlineLambda);
    
    inlineLambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['batch:SubmitJob'],
        resources: [props.renderJobQueue.jobQueueArn, props.renderJobDefinition.jobDefinitionArn],
      }),
    );

    this.lambda = inlineLambda;
  }
}
