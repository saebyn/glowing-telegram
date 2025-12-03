import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import type { ITable } from 'aws-cdk-lib/aws-dynamodb';
import type { IBucket } from 'aws-cdk-lib/aws-s3';
import { InputType } from 'aws-cdk-lib/aws-stepfunctions';
import * as sfn from 'aws-cdk-lib/aws-stepfunctions';
import * as events from 'aws-cdk-lib/aws-events';
import {
  EcsFargateContainerDefinition,
  EcsJobDefinition,
  type IJobQueue,
} from 'aws-cdk-lib/aws-batch';
import type { IEventBus } from 'aws-cdk-lib/aws-events';
import { EcrImage, LogDrivers } from 'aws-cdk-lib/aws-ecs';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as tasks from 'aws-cdk-lib/aws-stepfunctions-tasks';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import type TaskMonitoringConstruct from './taskMonitoring';
import ServiceLambdaConstruct, { LOG_GROUP_PREFIX, LOG_RETENTION } from './util/serviceLambda';

// Constants for upload status, should match the `UploadStatus` type in .../types/src/types.ts
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
  readonly youtubeUserSecretBasePath: string;
  readonly taskMonitoring: TaskMonitoringConstruct;
  readonly imageVersion?: string;
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
      youtubeUserSecretBasePath,
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
              resourceName: `${youtubeUserSecretBasePath}/*`,
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

    // Create log group for YouTube uploader batch job
    const uploadVideoLogGroup = new logs.LogGroup(this, 'UploadVideoLogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/batch/upload-video`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    const containerDefinition = new EcsFargateContainerDefinition(
      this,
      'UploadVideoContainer',
      {
        image: EcrImage.fromEcrRepository(repo, props.imageVersion || 'latest'),
        cpu: 1,
        command: ['Ref::episode_id'],
        assignPublicIp: true,
        memory: cdk.Size.gibibytes(2),
        environment: {
          EPISODE_RENDER_BUCKET: mediaOutputBucket.bucketName,
          EPISODE_TABLE_NAME: episodeTable.tableName,
          USER_SECRET_PATH: props.youtubeUserSecretBasePath,
          YOUTUBE_SECRET_ARN: youtubeAppSecret.secretArn,
          MAX_RETRY_SECONDS: '3600',
          USER_AGENT: 'glowing-telegram/1.0',
          RUST_LOG: 'info',
        },
        executionRole,
        jobRole,
        logging: LogDrivers.awsLogs({
          streamPrefix: 'upload-video',
          logGroup: uploadVideoLogGroup,
        }),
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
        payload: sfn.TaskInput.fromObject({
          'episode_id.$': '$.id',
        }),
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
            cdk.aws_stepfunctions.JsonPath.stringAt('$.id'),
          ),
        },
        projectionExpression: [
          new tasks.DynamoProjectionExpression().withAttribute('user_id'),
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

    const notifySuccessState = new tasks.EventBridgePutEvents(
      this,
      'NotifySuccessState',
      {
        entries: [
          {
            eventBus: eventBus,
            detailType: 'EpisodeUploadStatus',
            detail: {
              type: InputType.OBJECT,
              value: {
                status: 'SUCCEEDED',
                episodeId: cdk.aws_stepfunctions.JsonPath.stringAt('$.id'),
                userId: cdk.aws_stepfunctions.JsonPath.stringAt(
                  '$.uploadVideoResult.Item.user_id.S',
                ),
              },
            },
            source: 'glowing-telegram.youtube-uploader',
          },
        ],
        integrationPattern:
          cdk.aws_stepfunctions.IntegrationPattern.REQUEST_RESPONSE,
      },
    );

    const notifyFailureState = new tasks.EventBridgePutEvents(
      this,
      'NotifyFailureState',
      {
        entries: [
          {
            eventBus: eventBus,
            detailType: 'EpisodeUploadStatus',
            detail: {
              type: InputType.OBJECT,
              value: {
                status: 'FAILED',
                userId: cdk.aws_stepfunctions.JsonPath.stringAt(
                  '$.uploadVideoResult.Item.user_id.S',
                ),
                episodeId: cdk.aws_stepfunctions.JsonPath.stringAt('$.id'),
                errorMessage: cdk.aws_stepfunctions.JsonPath.stringAt(
                  '$.uploadVideoResult.Item.error_message.S',
                ),
              },
            },
            source: 'glowing-telegram.youtube-uploader',
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
            cdk.aws_stepfunctions.JsonPath.stringAt('$.id'),
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

    markAsUploadedState.next(notifySuccessState);

    const markAsNotReadyToUploadState = new tasks.DynamoUpdateItem(
      this,
      'MarkAsNotReadyToUploadState',
      {
        table: episodeTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            cdk.aws_stepfunctions.JsonPath.stringAt('$.id'),
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
        iamResources: [
          episodeTable.tableArn,
          `${episodeTable.tableArn}/index/upload_status-upload_queue_timestamp-index`,
        ],
        resultPath: '$.episodes',
      },
    );

    const episodeProcessor = sfn.Chain.start(uploadVideoState)
      .next(getUploadResultState)
      .next(
        new sfn.Choice(this, 'UploadSuccess?')
          .when(
            sfn.Condition.stringEquals(
              '$.uploadVideoResult.Item.upload_status.S',
              UPLOAD_FAILED,
            ),
            markAsNotReadyToUploadState.next(notifyFailureState),
          )
          .when(
            sfn.Condition.stringEquals(
              '$.uploadVideoResult.Item.upload_status.S',
              UPLOAD_THROTTLED,
            ),
            new sfn.Wait(this, 'WaitForRetry', {
              comment: 'Wait for the amount of time specified in the response',
              time: sfn.WaitTime.secondsPath(
                '$.uploadVideoResult.Item.retry_after_seconds.N',
              ),
            }).next(uploadVideoState),
          )
          .otherwise(markAsUploadedState),
      );

    const stepFunctionDefinition = sfn.Chain.start(queryEpisodeState).next(
      new sfn.Map(this, 'ForEachEpisode', {
        itemsPath: '$.episodes.Items',
        itemSelector: {
          id: sfn.JsonPath.stringAt('$$.Map.Item.Value.id.S'),
        },
        resultPath: sfn.JsonPath.DISCARD,
        maxConcurrency: 1,
      }).itemProcessor(episodeProcessor),
    );

    // Create log group for YouTube Uploader Step Functions state machine
    const stepFunctionLogGroup = new logs.LogGroup(
      this,
      'YoutubeUploaderStateMachineLogGroup',
      {
        logGroupName: `${LOG_GROUP_PREFIX}/stepfunctions/youtube-uploader`,
        retention: LOG_RETENTION,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      },
    );

    const stepFunction = new sfn.StateMachine(
      this,
      'YoutubeUploaderStateMachine',
      {
        definitionBody: sfn.DefinitionBody.fromChainable(
          stepFunctionDefinition,
        ),
        timeout: cdk.Duration.hours(1),
        logs: {
          destination: stepFunctionLogGroup,
          level: sfn.LogLevel.ERROR,
          includeExecutionData: true,
        },
      },
    );

    this.apiLambda = new ServiceLambdaConstruct(this, 'YoutubeUploaderApiLambda', {
      lambdaOptions: {
        timeout: cdk.Duration.seconds(30),
        environment: {
          EPISODES_TABLE_NAME: episodeTable.tableName,
          STEPFUNCTION_ARN: stepFunction.stateMachineArn,
          EVENT_BUS_NAME: eventBus.eventBusName,
          USER_SECRET_PATH: youtubeUserSecretBasePath,
          YOUTUBE_SECRET_ARN: youtubeAppSecret.secretArn,
          UPLOAD_READY_TO_UPLOAD: UPLOAD_READY_TO_UPLOAD,
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
          new iam.PolicyStatement({
            actions: ['events:PutEvents'],
            resources: [eventBus.eventBusArn],
          }),
          new iam.PolicyStatement({
            actions: ['secretsmanager:GetSecretValue'],
            resources: [
              youtubeAppSecret.secretArn,
              cdk.Arn.format(
                {
                  service: 'secretsmanager',
                  resource: 'secret',
                  resourceName: `${youtubeUserSecretBasePath}/*`,
                  arnFormat: cdk.ArnFormat.COLON_RESOURCE_NAME,
                },
                cdk.Stack.of(this),
              ),
            ],
          }),
        ],
      },
      name: 'youtube-uploader-lambda',
      imageVersion: props.imageVersion,
    }).lambda;

    new events.Rule(this, 'UploadCompleteEventRule', {
      eventBus,
      eventPattern: {
        source: ['glowing-telegram.youtube-uploader'],
        detailType: ['EpisodeUploadStatus'],
      },
      enabled: true,
    }).addTarget(
      props.taskMonitoring.newEventTarget({
        name: events.EventField.fromPath('$.detail.episodeId'),
        status: events.EventField.fromPath('$.detail.status'),
        time: events.EventField.fromPath('$.time'),
        task_type: 'upload',
        record_id: events.EventField.fromPath('$.detail.episodeId'),
        user_id: events.EventField.fromPath('$.detail.userId'),
      }),
    );
  }
}
