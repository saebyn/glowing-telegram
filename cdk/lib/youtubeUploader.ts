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
import * as tasks from 'aws-cdk-lib/aws-stepfunctions-tasks';

type YoutubeUploaderProps = {
  readonly mediaOutputBucket: IBucket;
  readonly episodeTable: ITable;
  readonly jobQueue: IJobQueue;
  readonly eventBus: IEventBus;
};

export default class YoutubeUploader extends Construct {
  readonly stepFunction: StateMachine;
  private readonly uploadVideoJob: EcsJobDefinition;

  constructor(scope: Construct, id: string, props: YoutubeUploaderProps) {
    super(scope, id);

    const { mediaOutputBucket, episodeTable, jobQueue, eventBus } = props;

    const uploadVideoExecutionRole = new cdk.aws_iam.Role(
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

    const uploadVideoContainerImage = EcrImage.fromEcrRepository(
      cdk.aws_ecr.Repository.fromRepositoryName(
        this,
        'UploadVideoContainerImage',
        'glowing-telegram/upload-video',
      ),
    );

    const containerDefinition = new EcsFargateContainerDefinition(
      this,
      'UploadVideoContainer',
      {
        image: uploadVideoContainerImage,
        cpu: 1,
        memory: cdk.Size.gibibytes(2),
        environment: {
          MEDIA_OUTPUT_BUCKET_NAME: mediaOutputBucket.bucketName,
          EPISODE_TABLE_NAME: episodeTable.tableName,
          EVENT_BUS_NAME: eventBus.eventBusName,
        },
        executionRole: uploadVideoExecutionRole,
      },
    );

    this.uploadVideoJob = new EcsJobDefinition(this, 'UploadVideoJob', {
      jobDefinitionName: 'UploadVideo',
      container: containerDefinition,
      timeout: cdk.Duration.hours(1),
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
          ':status': tasks.DynamoAttributeValue.fromString('uploaded'),
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
            'not_ready_to_upload',
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
            ':status': { S: 'ready_to_upload' },
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
              'FAILED',
            ),
            markAsNotReadyToUploadState.next(notifyFailureState),
          )
          .when(
            sfn.Condition.stringEquals(
              '$.uploadVideoResult.upload_status',
              'THROTTLED',
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

    this.stepFunction = new sfn.StateMachine(
      this,
      'YoutubeUploaderStateMachine',
      {
        definitionBody: sfn.DefinitionBody.fromChainable(
          stepFunctionDefinition,
        ),
        timeout: cdk.Duration.hours(1),
      },
    );
  }
}
