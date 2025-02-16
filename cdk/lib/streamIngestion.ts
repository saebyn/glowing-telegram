import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import type * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import type * as lambda from 'aws-cdk-lib/aws-lambda';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as stepfunctions from 'aws-cdk-lib/aws-stepfunctions';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as tasks from 'aws-cdk-lib/aws-stepfunctions-tasks';
import ServiceLambdaConstruct from './util/serviceLambda';

const INGESTION_VERSION = 'v1.0.0';

interface StreamIngestionConstructProps {
  audioTranscriberJob: batch.IJobDefinition;
  videoIngesterJob: batch.IJobDefinition;

  cpuBatchJobQueue: batch.IJobQueue;
  gpuBatchJobQueue: batch.IJobQueue;

  videoMetadataTable: dynamodb.ITable;
  streamsTable: dynamodb.ITable;

  videoArchive: s3.IBucket;

  openaiSecret: secretsmanager.ISecret;
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
        iterator: { index: 0 },
        context: {
          'transcription.$': '$.initialPrompt',
          'summarization.$': '$.initialSummary',
        },
        'stream_id.$': '$.streamId',
      },
    });

    const getStreamFromDynamoDB = new tasks.DynamoGetItem(
      this,
      'Get stream from DynamoDB',
      {
        comment: 'Get the stream record from the DynamoDB table',
        table: props.streamsTable,
        key: {
          id: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.stream_id'),
          ),
        },
        projectionExpression: [
          new tasks.DynamoProjectionExpression().withAttribute('id'),
          new tasks.DynamoProjectionExpression().withAttribute('prefix'),
          new tasks.DynamoProjectionExpression().withAttribute(
            'ingestion_version',
          ),
        ],
        resultPath: '$.streamRecord',
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
        'streamRecord.$': '$.streamRecord',
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
          new tasks.DynamoProjectionExpression().withAttribute('transcription'),
        ],
        expressionAttributeNames: { '#k': 'key' },
        resultPath: '$.dynamodb',
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
      .otherwise(new stepfunctions.Succeed(this, 'Success'));

    const summarizeTranscriptionTask = new tasks.LambdaInvoke(
      this,
      'Summarize Transcription',
      {
        comment: 'Invoke the summarizeTranscription Lambda',
        lambdaFunction: props.summarizeTranscription,
        payload: stepfunctions.TaskInput.fromObject({
          input_key: stepfunctions.JsonPath.stringAt('$.dynamodb.Item.key.S'),
          transcription: stepfunctions.JsonPath.stringAt(
            '$.dynamodb.Item.transcription.M.segments',
          ),
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

    const updateMetadata = new tasks.DynamoUpdateItem(
      this,
      'Update stream_id in metadata',
      {
        table: props.videoMetadataTable,
        key: {
          key: tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.dynamodb.Item.key.S'),
          ),
        },
        updateExpression:
          'SET stream_id = :streamId, video_clip_count = :videoClipCount, ingestion_version = :ingestionVersion',
        expressionAttributeValues: {
          ':streamId': tasks.DynamoAttributeValue.fromString(
            stepfunctions.JsonPath.stringAt('$.stream_id'),
          ),
          ':videoClipCount': tasks.DynamoAttributeValue.fromNumber(
            stepfunctions.JsonPath.numberAt('$.count'),
          ),
          ':ingestionVersion':
            tasks.DynamoAttributeValue.fromString(INGESTION_VERSION),
        },
        resultPath: stepfunctions.JsonPath.DISCARD,
      },
    );

    const incrementIndex = new stepfunctions.Pass(this, 'Increment index', {
      comment: 'Increment the iterator index',
      parameters: {
        'index.$': 'States.MathAdd(1, $.iterator.index)',
      },
      resultPath: '$.iterator',
    });

    const doneWithIngest = new stepfunctions.Succeed(
      this,
      'Done with ingest for this video',
    );

    const ingestAllVideos = new stepfunctions.Map(this, 'Ingest all videos', {
      maxConcurrency: 10,
      itemsPath: '$.videoKeys',
      itemSelector: {
        'key.$': '$$.Map.Item.Value',
        'context.$': '$.context',
        'streamRecord.$': '$.streamRecord',
      },
    }).itemProcessor(
      new stepfunctions.Choice(
        this,
        'Check stream was ingested with correct version',
      )
        .when(
          stepfunctions.Condition.and(
            stepfunctions.Condition.isPresent(
              '$.streamRecord.Item.ingestion_version.S',
            ),
            stepfunctions.Condition.stringEquals(
              '$.streamRecord.Item.ingestion_version.S',
              INGESTION_VERSION,
            ),
          ),
          doneWithIngest,
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
          })

            .next(
              new tasks.DynamoGetItem(
                this,
                'GetItem from DynamoDB with audio',
                {
                  table: props.videoMetadataTable,
                  key: {
                    key: tasks.DynamoAttributeValue.fromString(
                      stepfunctions.JsonPath.stringAt('$.key'),
                    ),
                  },
                  projectionExpression: [
                    new tasks.DynamoProjectionExpression().withAttribute('#k'),
                    new tasks.DynamoProjectionExpression().withAttribute(
                      'audio',
                    ),
                    new tasks.DynamoProjectionExpression().withAttribute(
                      'transcription',
                    ),
                  ],
                  expressionAttributeNames: { '#k': 'key' },
                  resultPath: '$.dynamodb',
                },
              ),
            )
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
            .next(doneWithIngest),
        ),
    );

    const chain = setUpState
      .next(getStreamFromDynamoDB)
      .next(listVideoObjects)
      .next(parseVideoKeys)
      .next(ingestAllVideos)
      .next(loopOverVideos.afterwards());

    getItemFromDynamoDB
      .next(summarizeTranscriptionTask)
      .next(updateMetadata)
      .next(incrementIndex)
      .next(loopOverVideos);

    return stepfunctions.DefinitionBody.fromChainable(chain);
  }
}
