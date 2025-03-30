import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as events from 'aws-cdk-lib/aws-events';
import * as event_targets from 'aws-cdk-lib/aws-events-targets';
import * as logs from 'aws-cdk-lib/aws-logs';
import type * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as stepfunctions from 'aws-cdk-lib/aws-stepfunctions';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as tasks from 'aws-cdk-lib/aws-stepfunctions-tasks';
import type * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import ServiceLambdaConstruct from './util/serviceLambda';
import type TaskMonitoringConstruct from './taskMonitoring';

const INGESTION_VERSION = 'v1.0.0';

interface StreamIngestionConstructProps {
  taskMonitoring: TaskMonitoringConstruct;
  audioTranscriberJob: batch.IJobDefinition;
  videoIngesterJob: batch.IJobDefinition;

  cpuBatchJobQueue: batch.IJobQueue;
  gpuBatchJobQueue: batch.IJobQueue;

  videoMetadataTable: dynamodb.ITable;
  streamsTable: dynamodb.ITable;

  videoArchive: s3.IBucket;

  openaiSecret: secretsmanager.ISecret;

  mediaDistribution: cloudfront.Distribution;
}

export default class StreamIngestionConstruct extends Construct {
  stepFunction: stepfunctions.StateMachine;

  constructor(
    scope: Construct,
    id: string,
    props: StreamIngestionConstructProps,
  ) {
    super(scope, id);

    const summarizeTranscription = new ServiceLambdaConstruct(
      this,
      'SummarizeTranscription',
      {
        name: 'summarize-transcription-lambda',
        lambdaOptions: {
          timeout: cdk.Duration.minutes(15),
          environment: {
            OPENAI_SECRET_ARN: props.openaiSecret.secretArn,
            METADATA_TABLE_NAME: props.videoMetadataTable.tableName,
            OPENAI_MODEL: 'gpt-4o-2024-11-20',
            OPENAI_INSTRUCTIONS: `
Generate a detailed summary report for the given transcript of a 20-minute video, using the provided context summary of preceding videos to enhance continuity and depth.

The summary you generate must be not only informational for content review but also reusable for future summarization and reference purposes. Combine the details from the current video with the larger context of the ongoing series to identify recurring themes, connections, and key points.

# Steps
1. **Analyze the Transcript**: Read the 20-minute transcript thoroughly to capture major discussion points, arguments, examples, questions, and any pivotal moments or insights, and noting the time periods of each.
2. **Incorporate Preceding Context**: Use the summary of the preceding videos to identify overarching topics, common themes, recurring elements, and key progressions in the narrative.
3. **Extract Key Points**: Highlight:
   - The main topics covered in the current video.
   - Key arguments or perspectives.
   - Examples or anecdotes that have importance.
   - How the discussion connects to or extends previous episodes.
4. **Generate the Output**:
   - Create a high-level summary of the current video.
   - Note connections to previous videos, showing continuity of ideas and context where applicable.
   - Identify questions introduced or resolved, transitions in focus, or shifts from the previous video.
   - Highlight significant new points or insights and how they enhance the larger theme.
   - Review any errors or inconsistencies in the transcript that need clarification or correction (attentions).
   - Identify any gaffs or issues that might require further investigation or follow-up (transcript errors).

# Notes 
- Ensure continuity between videos by emphasizing the ongoing build of ideas.
- Focus on the usefulness of the \`summary_context\` in shaping future summaries, noting key phrases, themes, or topics that might resurface or require revisiting.`,
          },
        },
      },
    );

    props.videoMetadataTable.grantReadWriteData(summarizeTranscription.lambda);
    props.openaiSecret.grantRead(summarizeTranscription.lambda);

    this.stepFunction = new stepfunctions.StateMachine(
      this,
      'StreamIngestionStateMachine',
      {
        tracingEnabled: true,
        stateMachineType: stepfunctions.StateMachineType.STANDARD,
        definitionBody: this.stateMachine({
          summarizeTranscription: summarizeTranscription.lambda,
          ...props,
        }),
      },
    );

    props.videoMetadataTable.grantReadWriteData(this.stepFunction);
    props.streamsTable.grantReadWriteData(this.stepFunction);

    this.stepFunction.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['batch:SubmitJob', 'batch:DescribeJobs'],
        resources: [
          props.cpuBatchJobQueue.jobQueueArn,
          props.gpuBatchJobQueue.jobQueueArn,
          props.audioTranscriberJob.jobDefinitionArn,
          props.videoIngesterJob.jobDefinitionArn,
        ],
      }),
    );

    this.stepFunction.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['batch:TerminateJob'],
        resources: ['*'],
      }),
    );

    const stepfunctionStatusEventLambda = new lambda.Function(
      this,
      'StepFunctionStatusEventLambda',
      {
        code: lambda.Code.fromInline(`
import json
import boto3

def handler(event, context):
  events = boto3.client('events')

  input = json.loads(event['input'])
  
  events.put_events(
    Entries=[
      {
        'Source': 'glowing-telegram.stream-ingestion',
        'DetailType': 'StreamIngestionStatus',
        'Detail': json.dumps({
          'status': event['status'],
          'name': event['name'],
          'stream_id': input['stream_id'],
          'user_id': input['user_id'],
        }),
      }
    ]
  )
`),
        handler: 'index.handler',
        runtime: lambda.Runtime.PYTHON_3_13,
        tracing: lambda.Tracing.ACTIVE,
        logRetention: logs.RetentionDays.ONE_WEEK,
        loggingFormat: lambda.LoggingFormat.JSON,
      },
    );

    stepfunctionStatusEventLambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['events:PutEvents'],
        resources: ['*'],
      }),
    );

    new events.Rule(this, 'StepFunctionStatusRule', {
      eventPattern: {
        source: ['aws.states'],
        detailType: ['Step Functions Execution Status Change'],
        detail: {
          stateMachineArn: [this.stepFunction.stateMachineArn],
        },
      },
      enabled: true,
    }).addTarget(
      new event_targets.LambdaFunction(stepfunctionStatusEventLambda),
    );

    new events.Rule(this, 'StreamIngestionEventRule', {
      eventPattern: {
        source: ['glowing-telegram.stream-ingestion'],
        detailType: ['StreamIngestionStatus'],
      },
      enabled: true,
    }).addTarget(
      props.taskMonitoring.newEventTarget({
        name: events.EventField.fromPath('$.detail.name'),
        status: events.EventField.fromPath('$.detail.status'),
        time: events.EventField.fromPath('$.time'),
        task_type: 'ingestion',
        record_id: events.EventField.fromPath('$.detail.stream_id'),
        user_id: events.EventField.fromPath('$.detail.user_id'),
      }),
    );
  }

  stateMachine(
    props: StreamIngestionConstructProps & {
      summarizeTranscription: lambda.IFunction;
    },
  ): stepfunctions.DefinitionBody {
    const setUpState = new stepfunctions.Pass(this, 'Set up state', {
      comment:
        'Set up the initial state with the iterator, context, and stream ID',
      parameters: {
        iterator: { index: 0, start_time: 0 },
        context: {
          'transcription.$': '$.initialPrompt',
          'summarization.$': '$.initialSummary',
        },
        'stream_id.$': '$.streamId',
        'user_id.$': '$.userId',
      },
    });

    const sendStartEvent = new tasks.EventBridgePutEvents(
      this,
      'Send start event',
      {
        entries: [
          {
            detailType: 'StreamIngestionStatus',
            source: 'glowing-telegram.stream-ingestion',
            detail: stepfunctions.TaskInput.fromObject({
              stream_id: stepfunctions.JsonPath.stringAt('$.stream_id'),
              user_id: stepfunctions.JsonPath.stringAt('$.user_id'),
              name: stepfunctions.JsonPath.executionName,
              status: 'RUNNING',
            }),
          },
        ],

        resultPath: stepfunctions.JsonPath.DISCARD,
      },
    );

    const getStreamFromDynamoDB = new tasks.DynamoGetItem(
      this,
      'Get stream from DynamoDB',
      {
        comment: 'Get the stream from DynamoDB',
        table: props.streamsTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.stream_id'),
          ),
        },
        projectionExpression: [
          new tasks.DynamoProjectionExpression().withAttribute('prefix'),
        ],
        resultPath: '$.streamRecord',
      },
    );

    const updateStreamRecord = new tasks.DynamoUpdateItem(
      this,
      'Update stream record',
      {
        comment: 'Update the video clip count in the stream record',
        table: props.streamsTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.stream_id'),
          ),
        },
        updateExpression: 'SET #s = :s',
        expressionAttributeNames: { '#s': 'video_clip_count' },
        expressionAttributeValues: {
          ':s': tasks.DynamoAttributeValue.numberFromString(
            stepfunctions.JsonPath.format(
              '{}',
              stepfunctions.JsonPath.stringAt('$.count'),
            ),
          ),
        },
        resultPath: stepfunctions.JsonPath.DISCARD,
      },
    );

    const listVideoObjects = new tasks.CallAwsService(
      this,
      'List video objects',
      {
        comment: 'List S3 objects for the given prefix',
        service: 's3',
        action: 'listObjectsV2',
        parameters: {
          Bucket: props.videoArchive.bucketName,
          Prefix: stepfunctions.JsonPath.stringAt(
            '$.streamRecord.Item.prefix.S',
          ),
        },
        iamAction: 's3:ListBucket',
        iamResources: [props.videoArchive.bucketArn],
        resultPath: '$.listResult',
      },
    );

    const parseVideoKeys = new stepfunctions.Pass(this, 'Parse video keys', {
      comment: 'Parse the video keys and set the iterator count',
      parameters: {
        'videoKeys.$': '$.listResult.Contents[*].Key',
        'count.$': 'States.ArrayLength($.listResult.Contents[*].Key)',
        'context.$': '$.context',
        'stream_id.$': '$.stream_id',
        'iterator.$': '$.iterator',
      },
      resultPath: '$',
    });

    const getItemFromDynamoDB = new tasks.DynamoGetItem(
      this,
      'GetItem from DynamoDB',
      {
        table: props.videoMetadataTable,
        key: {
          key: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt(
              'States.ArrayGetItem($.videoKeys, $.iterator.index)',
            ),
          ),
        },
        projectionExpression: [
          new tasks.DynamoProjectionExpression().withAttribute('#k'),
          new tasks.DynamoProjectionExpression().withAttribute('audio'),
        ],
        expressionAttributeNames: { '#k': 'key' },
        resultPath: '$.dynamodb',
      },
    );

    const saveStartTimeToDynamoDB = new tasks.DynamoUpdateItem(
      this,
      'Save start time to DynamoDB',
      {
        table: props.videoMetadataTable,
        key: {
          key: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.dynamodb.Item.key.S'),
          ),
        },
        updateExpression: 'SET start_time = :startTime',
        expressionAttributeValues: {
          ':startTime': tasks.DynamoAttributeValue.numberFromString(
            stepfunctions.JsonPath.format(
              '{}',
              stepfunctions.JsonPath.stringAt('$.iterator.start_time'),
            ),
          ),
        },
        resultPath: stepfunctions.JsonPath.DISCARD,
      },
    );

    const invalidatePlaylistCache = new tasks.CallAwsService(
      this,
      'Invalidate CloudFront Distribution',
      {
        service: 'cloudfront',
        action: 'createInvalidation',
        parameters: {
          DistributionId: props.mediaDistribution.distributionId,
          InvalidationBatch: {
            Paths: {
              Quantity: 1,
              Items: stepfunctions.JsonPath.array(
                stepfunctions.JsonPath.format(
                  '/playlist/{}.m3u8',
                  stepfunctions.JsonPath.stringAt('$.stream_id'),
                ),
              ),
            },
            CallerReference: stepfunctions.JsonPath.format(
              '{}-{}',
              stepfunctions.JsonPath.stringAt('$.stream_id'),
              stepfunctions.JsonPath.stateEnteredTime,
            ),
          },
        },
        iamAction: 'cloudfront:CreateInvalidation',
        iamResources: [
          cdk.Stack.of(this).formatArn({
            service: 'cloudfront',
            resource: 'distribution',
            region: '',
            resourceName: props.mediaDistribution.distributionId,
          }),
        ],
        resultPath: stepfunctions.JsonPath.DISCARD,
      },
    );

    const loopOverVideos = new stepfunctions.Choice(this, 'Loop over Videos')
      .when(
        stepfunctions.Condition.numberLessThanJsonPath(
          '$.iterator.index',
          '$.count',
        ),
        getItemFromDynamoDB,
      )
      .otherwise(
        invalidatePlaylistCache.next(
          new stepfunctions.Succeed(this, 'Success'),
        ),
      );

    const summarizeTranscriptionTask = new tasks.LambdaInvoke(
      this,
      'Summarize Transcription',
      {
        comment: 'Invoke the summarizeTranscription Lambda',
        lambdaFunction: props.summarizeTranscription,
        payload: stepfunctions.TaskInput.fromObject({
          input_key: stepfunctions.JsonPath.stringAt('$.dynamodb.Item.key.S'),
          transcription_context: stepfunctions.JsonPath.stringAt(
            '$.context.transcription',
          ),
          summarization_context: stepfunctions.JsonPath.stringAt(
            '$.context.summarization',
          ),
        }),
        resultSelector: {
          'summarization.$': '$.Payload.summarization_context',
          'transcription.$': '$.Payload.transcription_context',
        },
        resultPath: '$.context',
      },
    );

    const incrementIndex = new stepfunctions.Pass(this, 'Increment index', {
      comment: 'Increment the iterator index',
      parameters: {
        'index.$': 'States.MathAdd(1, $.iterator.index)',
        'start_time.$':
          'States.MathAdd($.iterator.start_time, States.StringToJson($.dynamodb.Item.metadata.M.format.M.duration.N))',
      },
      resultPath: '$.iterator',
    });

    const ingestAllVideos = new stepfunctions.Map(this, 'Ingest all videos', {
      itemsPath: '$.videoKeys',
      itemSelector: {
        'key.$': '$$.Map.Item.Value',
        'stream_id.$': '$.stream_id',
        'count.$': '$.count',
      },
      resultPath: stepfunctions.JsonPath.DISCARD,
    }).itemProcessor(
      new tasks.DynamoGetItem(this, 'Get video metadata', {
        table: props.videoMetadataTable,
        key: {
          key: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.key'),
          ),
        },
        projectionExpression: [
          new tasks.DynamoProjectionExpression().withAttribute('#k'),
          new tasks.DynamoProjectionExpression().withAttribute(
            'ingestion_version',
          ),
        ],
        expressionAttributeNames: { '#k': 'key' },
        resultPath: '$.dynamodb',
      })
        .next(
          new stepfunctions.Choice(
            this,
            'Check video was ingested with correct version',
          )
            .when(
              stepfunctions.Condition.and(
                stepfunctions.Condition.isPresent(
                  '$.dynamodb.Item.ingestion_version.S',
                ),
                stepfunctions.Condition.stringEquals(
                  '$.dynamodb.Item.ingestion_version.S',
                  INGESTION_VERSION,
                ),
              ),
              new stepfunctions.Pass(this, 'Skip ingestion', {
                comment:
                  'Skip ingestion if video was ingested with correct version',
              }),
              {
                comment: 'Correct version',
              },
            )
            .otherwise(
              new tasks.BatchSubmitJob(this, 'Ingest Video', {
                jobDefinitionArn: props.videoIngesterJob.jobDefinitionArn,
                jobQueueArn: props.cpuBatchJobQueue.jobQueueArn,
                payload: stepfunctions.TaskInput.fromObject({
                  'key.$': '$.key',
                }),
                jobName: 'ingest-video',
                resultPath: stepfunctions.JsonPath.DISCARD,
              }),
            )
            .afterwards(),
        )
        .next(
          new tasks.DynamoUpdateItem(this, 'Update video metadata', {
            table: props.videoMetadataTable,
            key: {
              key: tasks.DynamoAttributeValue.fromString(
                stepfunctions.JsonPath.stringAt('$.key'),
              ),
            },
            updateExpression:
              'SET stream_id = :streamId, ingestion_version = :ingestionVersion',
            expressionAttributeValues: {
              ':streamId': tasks.DynamoAttributeValue.fromString(
                stepfunctions.JsonPath.stringAt('$.stream_id'),
              ),
              ':ingestionVersion':
                tasks.DynamoAttributeValue.fromString(INGESTION_VERSION),
            },
            resultPath: stepfunctions.JsonPath.DISCARD,
          }),
        ),
    );

    const chain = setUpState
      .next(sendStartEvent)
      .next(getStreamFromDynamoDB)
      .next(listVideoObjects)
      .next(parseVideoKeys)
      .next(updateStreamRecord)
      .next(ingestAllVideos)
      .next(loopOverVideos.afterwards());

    getItemFromDynamoDB
      .next(saveStartTimeToDynamoDB)
      .next(
        new tasks.BatchSubmitJob(this, 'Transcribe Audio to Text', {
          jobName: 'transcribe-audio',
          jobDefinitionArn: props.audioTranscriberJob.jobDefinitionArn,
          jobQueueArn: props.gpuBatchJobQueue.jobQueueArn,
          payload: stepfunctions.TaskInput.fromObject({
            'input_key.$': '$.dynamodb.Item.audio.S',
            'item_key.$': '$.dynamodb.Item.key.S',
            language: 'en',
            'initial_prompt.$': '$.context.transcription',
          }),
          resultPath: stepfunctions.JsonPath.DISCARD,
        }),
      )
      .next(
        new tasks.DynamoGetItem(this, 'GetItem from DynamoDB with metadata', {
          table: props.videoMetadataTable,
          key: {
            key: tasks.DynamoAttributeValue.fromString(
              stepfunctions.JsonPath.stringAt('$.dynamodb.Item.key.S'),
            ),
          },
          projectionExpression: [
            new tasks.DynamoProjectionExpression().withAttribute('#k'),
            // extract the metadata for the duration so we can increment the start_time
            new tasks.DynamoProjectionExpression().withAttribute('metadata'),
          ],
          expressionAttributeNames: { '#k': 'key' },
          resultPath: '$.dynamodb',
        }),
      )
      .next(summarizeTranscriptionTask)
      .next(incrementIndex)
      .next(loopOverVideos);

    return stepfunctions.DefinitionBody.fromChainable(chain);
  }
}
