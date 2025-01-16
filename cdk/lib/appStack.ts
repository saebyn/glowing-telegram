import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';

import APIConstruct from './api';
import UserManagementConstruct from './userManagement';
import DatastoreConstruct from './datastore';
import AudioTranscriberJobConstruct from './batch/audioTranscriberJob';
import StreamIngestion from './streamIngestion';
import BatchEnvironmentConstruct from './batch/environment';
import VideoIngestorConstruct from './batch/videoIngestorJob';
import TaskMonitoringConstruct from './taskMonitoring';

export interface AppStackProps {
  frontendAssetBucket: s3.IBucket;
  frontendVersion: string;
}

export default class GtCdkStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: AppStackProps) {
    const { frontendAssetBucket, frontendVersion, ...restProps } = props;

    super(scope, id, restProps);

    const vpc = new ec2.Vpc(this, 'Vpc', {
      natGateways: 0,
      subnetConfiguration: [
        {
          cidrMask: 24,
          name: 'Public',
          subnetType: ec2.SubnetType.PUBLIC,
        },
        {
          cidrMask: 24,
          name: 'Private',
          subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS,
        },
      ],
    });

    const userManagement = new UserManagementConstruct(this, 'UserManagement');

    const dataStore = new DatastoreConstruct(this, 'Datastore');

    const openaiSecret = new secretsmanager.Secret(this, 'OpenAISecret', {
      description: 'OpenAI API key',
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const batchEnvironment = new BatchEnvironmentConstruct(
      this,
      'BatchEnvironment',
      {
        vpc,
      },
    );

    const audioTranscriber = new AudioTranscriberJobConstruct(
      this,
      'AudioTranscriberJob',
      {
        outputBucket: dataStore.outputBucket,
        videoMetadataTable: dataStore.videoMetadataTable,
      },
    );

    const videoIngester = new VideoIngestorConstruct(this, 'VideoIngesterJob', {
      jobQueue: batchEnvironment.cpuJobQueue,
      outputBucket: dataStore.outputBucket,
      videoMetadataTable: dataStore.videoMetadataTable,
      videoArchiveBucket: dataStore.videoArchive,
      enableAutomaticIngestion: true,
    });

    const streamIngestion = new StreamIngestion(this, 'StreamIngestion', {
      audioTranscriberJob: audioTranscriber.jobDefinition,
      videoIngesterJob: videoIngester.jobDefinition,
      cpuBatchJobQueue: batchEnvironment.cpuJobQueue,
      gpuBatchJobQueue: batchEnvironment.gpuJobQueue,
      videoMetadataTable: dataStore.videoMetadataTable,
      streamsTable: dataStore.streamsTable,
      videoArchive: dataStore.videoArchive,
      openaiSecret,
    });

    new APIConstruct(this, 'API', {
      streamIngestionFunction: streamIngestion.stepFunction,
      userPool: userManagement.userPool,
      userPoolClients: [userManagement.userPoolClient],
      openaiSecret,
      videoMetadataTable: dataStore.videoMetadataTable,
      streamsTable: dataStore.streamsTable,
      streamSeriesTable: dataStore.streamSeriesTable,
      episodesTable: dataStore.episodesTable,
      profilesTable: dataStore.profilesTable,
      tasksTable: dataStore.tasksTable,
    });

    new TaskMonitoringConstruct(this, 'TaskMonitoring', {
      tasksTable: dataStore.tasksTable,
      streamIngestionStepFunction: streamIngestion.stepFunction,
    });

    new cloudfront.Distribution(this, 'FrontendDistribution', {
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withBucketDefaults(frontendAssetBucket, {
          originPath: `/${frontendVersion}`,
        }),
      },
    });
  }
}
