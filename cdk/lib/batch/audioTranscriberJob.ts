import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import type * as efs from 'aws-cdk-lib/aws-efs';
import * as logs from 'aws-cdk-lib/aws-logs';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from '../util/serviceLambda';

interface AudioTranscriberJobConstructProps {
  outputBucket: s3.IBucket;
  videoMetadataTable: dynamodb.ITable;
  modelCacheFileSystem: efs.FileSystem;
  modelCacheAccessPoint: efs.AccessPoint;
  imageVersion?: string;
}

/**
 * Audio transcriber job construct for AWS Batch
 */
export default class AudioTranscriberJobConstruct extends Construct {
  jobDefinition: batch.IJobDefinition;

  constructor(
    scope: Construct,
    id: string,
    props: AudioTranscriberJobConstructProps,
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

    props.videoMetadataTable.grantReadWriteData(jobRole);
    props.outputBucket.grantRead(jobRole);

    // Grant EFS access to the job role
    props.modelCacheFileSystem.grantReadWrite(jobRole);

    const repo = ecr.Repository.fromRepositoryName(
      this,
      'AudioTranscriberJobRepository',
      'glowing-telegram/audio-transcription',
    );

    // Define EFS volume for model caching
    const modelCacheVolume = batch.EcsVolume.efs({
      name: 'model-cache',
      containerPath: '/mnt/efs/models',
      fileSystem: props.modelCacheFileSystem,
      accessPointId: props.modelCacheAccessPoint.accessPointId,
      rootDirectory: '/',
      enableTransitEncryption: true,
      useJobRole: true,
    });
    // Create log group for audio transcriber batch job
    const logGroup = new logs.LogGroup(this, 'LogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/batch/audio-transcriber`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    const containerDefinition = new batch.EcsEc2ContainerDefinition(
      this,
      'AudioTranscriberJobContainer',
      {
        cpu: 1,
        memory: cdk.Size.gibibytes(8),
        gpu: 1,

        image: ecs.ContainerImage.fromEcrRepository(repo, props.imageVersion || 'latest'),

        environment: {
          INPUT_BUCKET: props.outputBucket.bucketName,
          DYNAMODB_TABLE: props.videoMetadataTable.tableName,
          NVIDIA_DRIVER_CAPABILITIES: 'all',
          RUST_LOG: 'info',
          HF_HOME: '/mnt/efs/models',  // HuggingFace model cache directory on EFS
        },

        volumes: [modelCacheVolume],

        command: [
          'Ref::item_key',
          'Ref::input_key',
          'Ref::initial_prompt',
          'Ref::language',
        ],

        executionRole,

        jobRole,

        logging: ecs.LogDrivers.awsLogs({
          streamPrefix: 'audio-transcriber',
          logGroup,
        }),
      },
    );

    this.jobDefinition = new batch.EcsJobDefinition(
      this,
      'AudioTranscriberJobDefinition',
      {
        container: containerDefinition,
        timeout: cdk.Duration.minutes(15), // Increased from 5 to 15 minutes to allow for 10-minute Whisper timeout + overhead
        parameters: {
          item_key: '<item_key>',
          input_key: '<input_key>',
          initial_prompt: '<initial_prompt>',
          language: '<language>',
        },
        retryAttempts: 1,
      },
    );
  }
}
