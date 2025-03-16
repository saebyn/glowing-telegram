import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import type { ITable } from 'aws-cdk-lib/aws-dynamodb';
import type { IBucket } from 'aws-cdk-lib/aws-s3';
import { InputType, type StateMachine } from 'aws-cdk-lib/aws-stepfunctions';
import * as sfn from 'aws-cdk-lib/aws-stepfunctions';
import {
  EcsFargateContainerDefinition,
  EcsJobDefinition,
  type IJobQueue,
} from 'aws-cdk-lib/aws-batch';
import type { IEventBus } from 'aws-cdk-lib/aws-events';
import { EcrImage } from 'aws-cdk-lib/aws-ecs';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as tasks from 'aws-cdk-lib/aws-stepfunctions-tasks';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';

const UPLOAD_READY_TO_UPLOAD = 'ready_to_upload';
const UPLOAD_NOT_READY_TO_UPLOAD = 'not_ready_to_upload';
const UPLOAD_UPLOADED = 'uploaded';
const UPLOAD_FAILED = 'FAILED';
const UPLOAD_THROTTLED = 'THROTTLED';

type YoutubeUploaderProps = {
  readonly mediaOutputBucket: IBucket;
  readonly episodeTable: ITable;
  readonly jobQueue: IJobQueue;
  readonly eventBus: IEventBus;
  readonly youtubeAppSecret: secretsmanager.ISecret;
};

export default class YoutubeUploader extends Construct {
  private readonly uploadVideoJob: EcsJobDefinition;
  public readonly apiLambda: lambda.IFunction;

  constructor(scope: Construct, id: string, props: YoutubeUploaderProps) {
    super(scope, id);

    const {
      mediaOutputBucket,
      episodeTable,
      jobQueue,
      eventBus,
      youtubeAppSecret,
    } = props;

    const executionRole = new cdk.aws_iam.Role(
      this,
      'UploadVideoExecutionRole',
      {
        assumedBy: new cdk.aws_iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
        managedPolicies: [
          cdk.aws_iam.ManagedPolicy.fromAwsManagedPolicyName(
            'service-role/AmazonECSTaskExecutionRolePolicy',
          ),
        ],
      },
    );

    const jobRole = new cdk.aws_iam.Role(this, 'UploadVideoJobRole', {
      assumedBy: new cdk.aws_iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
    });

    episodeTable.grantReadWriteData(jobRole);
    mediaOutputBucket.grantRead(jobRole);
    youtubeAppSecret.grantRead(jobRole);
    jobRole.addToPolicy(
      new iam.PolicyStatement({
        actions: [
          'secretsmanager:GetSecretValue',
          'secretsmanager:PutSecretValue',
        ],
        resources: [
          cdk.Arn.format(
            {
              service: 'secretsmanager',
              resource: 'secret',
              resourceName: 'gt/youtube/user/*',
              arnFormat: cdk.ArnFormat.COLON_RESOURCE_NAME,
            },
            cdk.Stack.of(this),
          ),
        ],
      }),
    );

    const repo = cdk.aws_ecr.Repository.fromRepositoryName(
      this,
      'UploadVideoContainerImage',
      'glowing-telegram/upload-video',
    );

    const containerDefinition = new EcsFargateContainerDefinition(
      this,
      'UploadVideoContainer',
      {
        image: EcrImage.fromEcrRepository(repo),
        cpu: 1,
        command: ['Ref::episode_id'],
        assignPublicIp: true,
        memory: cdk.Size.gibibytes(2),
        environment: {
          EPISODE_RENDER_BUCKET: mediaOutputBucket.bucketName,
          EPISODE_TABLE_NAME: episodeTable.tableName,
          USER_SECRET_PATH: 'gt/youtube/user',
          YOUTUBE_SECRET_ARN: youtubeAppSecret.secretArn,
          MAX_RETRY_SECONDS: '3600',
          USER_AGENT: 'glowing-telegram/1.0',
          RUST_LOG: 'info',
        },
        executionRole,
        jobRole,
      },
    );

    this.uploadVideoJob = new EcsJobDefinition(this, 'UploadVideoJob', {
      jobDefinitionName: 'UploadVideo',
      container: containerDefinition,
      timeout: cdk.Duration.hours(1),
      parameters: {
        episode_id: '<episode_id>',
      },
    });

    const uploadVideoState = new tasks.BatchSubmitJob(
      this,
      'UploadVideoState',
      {
        jobDefinitionArn: this.uploadVideoJob.jobDefinitionArn,
        jobName: 'UploadVideo',
        jobQueueArn: jobQueue.jobQueueArn,
        integrationPattern: cdk.aws_stepfunctions.IntegrationPattern.RUN_JOB,
        inputPath: '$.episode',
        resultPath: sfn.JsonPath.DISCARD,
      },
    );

    const getUploadResultState = new tasks.DynamoGetItem(
      this,
      'GetUploadResultState',
      {
        table: episodeTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            cdk.aws_stepfunctions.JsonPath.stringAt('$.episode.id'),
          ),
        },
        projectionExpression: [
          new tasks.DynamoProjectionExpression().withAttribute('upload_status'),
          new tasks.DynamoProjectionExpression().withAttribute('error_message'),
          new tasks.DynamoProjectionExpression().withAttribute(
            'retry_after_seconds',
          ),
        ],
        consistentRead: true,
        resultPath: '$.uploadVideoResult',
      },
    );

    const notifyFailureState = new tasks.EventBridgePutEvents(
      this,
      'NotifyFailureState',
      {
        entries: [
          {
            eventBus: eventBus,
            detailType: 'EpisodeUploadFailed',
            detail: {
              type: InputType.OBJECT,
              value: {
                episodeId:
                  cdk.aws_stepfunctions.JsonPath.stringAt('$.episode.id'),
                errorMessage: cdk.aws_stepfunctions.JsonPath.stringAt(
                  '$.uploadVideoResult.error_message',
                ),
              },
            },
            source: 'glowing-telegram',
          },
        ],
        integrationPattern:
          cdk.aws_stepfunctions.IntegrationPattern.REQUEST_RESPONSE,
      },
    );

    const markAsUploadedState = new tasks.DynamoUpdateItem(
      this,
      'MarkAsUploadedState',
      {
        table: episodeTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            cdk.aws_stepfunctions.JsonPath.stringAt('$.episode.id'),
          ),
        },
        updateExpression: 'SET #status = :status',
        expressionAttributeNames: {
          '#status': 'upload_status',
        },
        expressionAttributeValues: {
          ':status': tasks.DynamoAttributeValue.fromString(UPLOAD_UPLOADED),
        },
        resultPath: sfn.JsonPath.DISCARD,
      },
    );

    const markAsNotReadyToUploadState = new tasks.DynamoUpdateItem(
      this,
      'MarkAsNotReadyToUploadState',
      {
        table: episodeTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            cdk.aws_stepfunctions.JsonPath.stringAt('$.episode.id'),
          ),
        },
        updateExpression: 'SET #status = :status',
        expressionAttributeNames: {
          '#status': 'upload_status',
        },
        expressionAttributeValues: {
          ':status': tasks.DynamoAttributeValue.fromString(
            UPLOAD_NOT_READY_TO_UPLOAD,
          ),
        },
        resultPath: sfn.JsonPath.DISCARD,
      },
    );

    const queryEpisodeState = new tasks.CallAwsService(
      this,
      'QueryEpisodeState',
      {
        service: 'dynamodb',
        action: 'query',
        parameters: {
          TableName: episodeTable.tableName,
          IndexName: 'upload_status-upload_queue_timestamp-index',
          KeyConditionExpression: 'upload_status = :status',
          ExpressionAttributeValues: {
            ':status': { S: UPLOAD_READY_TO_UPLOAD },
          },
          ProjectionExpression: 'id',
          // This is working as a FIFO queue, so we want to process the oldest item first
          ScanIndexForward: true,
        },
        iamResources: [episodeTable.tableArn],
        resultPath: '$.episodes',
      },
    );

    const episodeProcessor = sfn.Chain.start(uploadVideoState)
      .next(getUploadResultState)
      .next(
        new sfn.Choice(this, 'UploadSuccess?')
          .when(
            sfn.Condition.stringEquals(
              '$.uploadVideoResult.upload_status',
              UPLOAD_FAILED,
            ),
            markAsNotReadyToUploadState.next(notifyFailureState),
          )
          .when(
            sfn.Condition.stringEquals(
              '$.uploadVideoResult.upload_status',
              UPLOAD_THROTTLED,
            ),
            new sfn.Wait(this, 'WaitForRetry', {
              comment: 'Wait for the amount of time specified in the response',
              time: sfn.WaitTime.secondsPath(
                '$.uploadVideoResult.retry_after_seconds',
              ),
            }).next(uploadVideoState),
          )
          .otherwise(markAsUploadedState),
      );

    const stepFunctionDefinition = sfn.Chain.start(queryEpisodeState).next(
      new sfn.Map(this, 'ForEachEpisode', {
        itemsPath: '$.episodes.Items',
        resultPath: '$.episode',
        maxConcurrency: 1,
      }).itemProcessor(episodeProcessor),
    );

    const stepFunction = new sfn.StateMachine(
      this,
      'YoutubeUploaderStateMachine',
      {
        definitionBody: sfn.DefinitionBody.fromChainable(
          stepFunctionDefinition,
        ),
        timeout: cdk.Duration.hours(1),
      },
    );

    this.apiLambda = new lambda.Function(this, 'YoutubeUploaderApiLambda', {
      runtime: lambda.Runtime.PYTHON_3_13,
      code: lambda.Code.fromInline(`
    import json
    import os
    import boto3
    from datetime import datetime, timedelta, timezone
    
    def handler(event, context):
        # Resource setup
        dynamodb = boto3.resource('dynamodb')
        sfn = boto3.client('stepfunctions')
        table = dynamodb.Table(os.environ['EPISODES_TABLE_NAME'])
        now = datetime.now(timezone.utc).isoformat()

        # Parse the event
        claims = event['requestContext']['authorizer']['claims']
        user_id = claims['sub']
        request_body = json.loads(event['body'])
        episode_ids = request_body.get('episode_ids', [])

        # Validate the input
        if not episode_ids:
            return {
                'statusCode': 400,
                'body': json.dumps('No episode IDs provided'),
            }
        if not all(isinstance(episode_id, str) for episode_id in episode_ids):
            return {
                'statusCode': 400,
                'body': json.dumps('Invalid episode IDs provided'),
            }
        if not user_id:
            return {
                'statusCode': 401,
                'body': json.dumps('Unauthorized'),
            }

        # Upload the records
        for episode_id in episode_ids:
          table.update_item(
              Key={'id': episode_id},
              UpdateExpression='SET #userId = :userId, #uploadStatus = :uploadStatus, #updatedAt = :now',
              ExpressionAttributeNames={'#userId': 'user_id', '#uploadStatus': 'upload_status', '#updatedAt': 'updated_at'},
              ExpressionAttributeValues={':userId': user_id, ':uploadStatus': '${UPLOAD_READY_TO_UPLOAD}', ':now': now},
          )

        # Check if the step function is already running
        response = sfn.list_executions(
            stateMachineArn=os.environ['STEPFUNCTION_ARN'],
            statusFilter='RUNNING'
        )
        if not response['executions']:
            # Start the step function execution if it's not running
            sfn.start_execution(
                stateMachineArn=os.environ['STEPFUNCTION_ARN'],
                input='{}'
            )

        return {
            'statusCode': 200,
            'body': '{}',
        }
    `),
      handler: 'index.handler',
      environment: {
        EPISODES_TABLE_NAME: episodeTable.tableName,
        STEPFUNCTION_ARN: stepFunction.stateMachineArn,
      },
      initialPolicy: [
        new iam.PolicyStatement({
          actions: ['dynamodb:UpdateItem'],
          resources: [episodeTable.tableArn],
        }),
        new iam.PolicyStatement({
          actions: ['states:StartExecution', 'states:ListExecutions'],
          resources: [stepFunction.stateMachineArn],
        }),
      ],
    });
  }
}
