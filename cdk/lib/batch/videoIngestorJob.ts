import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as sqs from 'aws-cdk-lib/aws-sqs';
import * as events from 'aws-cdk-lib/aws-events';
import * as eventsTargets from 'aws-cdk-lib/aws-events-targets';

interface VideoIngestorConstructProps {
  videoArchiveBucket: s3.IBucket;
  outputBucket: s3.IBucket;
  videoMetadataTable: dynamodb.ITable;

  jobQueue: batch.IJobQueue;

  enableAutomaticIngestion: boolean;
}

/**
 * Video ingestor job construct for AWS Batch
 */
export default class VideoIngestorConstruct extends Construct {
  jobDefinition: batch.IJobDefinition;

  constructor(
    scope: Construct,
    id: string,
    props: VideoIngestorConstructProps,
  ) {
    super(scope, id);

    const executionRole = new iam.Role(this, 'JobExecutionRole', {
      assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
      managedPolicies: [
        iam.ManagedPolicy.fromAwsManagedPolicyName(
          'service-role/AmazonECSTaskExecutionRolePolicy',
        ),
      ],
    });

    const jobRole = new iam.Role(this, 'JobRole', {
      assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
    });

    props.videoMetadataTable.grantWriteData(jobRole);
    props.videoArchiveBucket.grantRead(jobRole);
    props.outputBucket.grantWrite(jobRole);

    const repo = ecr.Repository.fromRepositoryName(
      this,
      'VideoIngestorJobRepository',
      'github/saebyn/glowing-telegram/video-ingestor',
    );

    const containerDefinition = new batch.EcsFargateContainerDefinition(
      this,
      'JobContainerDefinition',
      {
        cpu: 4,
        memory: cdk.Size.gibibytes(8),
        assignPublicIp: true,
        jobRole,
        executionRole,
        command: ['Ref::key'],
        image: ecs.ContainerImage.fromEcrRepository(repo, 'latest'),
        environment: {
          INPUT_BUCKET: props.videoArchiveBucket.bucketName,
          OUTPUT_BUCKET: props.outputBucket.bucketName,
          KEYFRAMES_PREFIX: 'keyframes',
          TRANSCODE_PREFIX: 'transcode',
          AUDIO_PREFIX: 'audio',
          DYNAMODB_TABLE: props.videoMetadataTable.tableName,
          SPEECH_TRACK_NUMBER: '2',
          NOISE_TOLERANCE: '0.004',
          SILENCE_DURATION: '30',
        },
      },
    );

    this.jobDefinition = new batch.EcsJobDefinition(this, 'JobDefinition', {
      container: containerDefinition,
      timeout: cdk.Duration.minutes(45),
      parameters: {
        key: '<key>',
      },
      retryAttempts: 1,
    });

    const videoUploadEventRule = new events.Rule(this, 'NewVideoRule', {
      eventPattern: {
        source: ['aws.s3'],
        detailType: ['Object Created'],
        detail: {
          bucket: {
            name: [props.videoArchiveBucket.bucketName],
          },
        },
      },
      enabled: props.enableAutomaticIngestion,
    });

    const deadLetterQueue = new sqs.Queue(this, 'DeadLetterQueue', {
      retentionPeriod: cdk.Duration.days(14),
    });

    videoUploadEventRule.addTarget(
      new eventsTargets.BatchJob(
        props.jobQueue.jobQueueArn,
        props.jobQueue,
        this.jobDefinition.jobDefinitionArn,
        this.jobDefinition,
        {
          deadLetterQueue,

          event: events.RuleTargetInput.fromObject({
            Parameters: {
              key: events.EventField.fromPath('$.detail.object.key'),
            },
          }),
        },
      ),
    );
  }
}
