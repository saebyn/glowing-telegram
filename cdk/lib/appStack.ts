import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';

import APIConstruct from './api';
import UserManagementConstruct from './userManagement';
import DatastoreConstruct from './datastore';
import AudioTranscriberJobConstruct from './batch/audioTranscriberJob';
import StreamIngestion from './streamIngestion';
import BatchEnvironmentConstruct from './batch/environment';
import VideoIngestorConstruct from './batch/videoIngestorJob';
import TaskMonitoringConstruct from './taskMonitoring';
import MediaServeConstruct from './mediaServeConstruct';
import RenderJobConstruct from './batch/renderJob';
import YoutubeUploader from './youtubeUploader';
import WebSocketAPIConstruct from './websocketApi';
import { EventBus } from 'aws-cdk-lib/aws-events';

interface AppStackProps extends cdk.StackProps {
  domainName: string;
}

export default class AppStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: AppStackProps) {
    const { domainName, ...restProps } = props;

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

    const userManagement = new UserManagementConstruct(this, 'UserManagement', {
      domainName,
    });

    const dataStore = new DatastoreConstruct(this, 'Datastore');

    const openaiSecret = new secretsmanager.Secret(this, 'OpenAISecret', {
      description: 'OpenAI API key',
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const youtubeAppSecret = new secretsmanager.Secret(
      this,
      'YoutubeAppSecret',
      {
        description: 'Youtube App Secret for API access in glowing-telegram',
        removalPolicy: cdk.RemovalPolicy.RETAIN,
      },
    );

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

    const mediaServe = new MediaServeConstruct(this, 'MediaServe', {
      mediaOutputBucket: dataStore.outputBucket,
      videoMetadataTable: dataStore.videoMetadataTable,
      domainName,
    });

    const taskMonitoring = new TaskMonitoringConstruct(this, 'TaskMonitoring', {
      tasksTable: dataStore.tasksTable,
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
      mediaDistribution: mediaServe.distribution,
      taskMonitoring,
    });

    const renderJob = new RenderJobConstruct(this, 'RenderJob', {
      inputBucket: dataStore.videoArchive,
      outputBucket: dataStore.outputBucket,
      episodeTable: dataStore.episodesTable,
      jobQueue: batchEnvironment.cpuJobQueue,
      taskMonitoring,
    });

    const youtubeUploader = new YoutubeUploader(this, 'YoutubeUploader', {
      episodeTable: dataStore.episodesTable,
      jobQueue: batchEnvironment.cpuJobQueue,
      mediaOutputBucket: dataStore.outputBucket,
      eventBus: EventBus.fromEventBusName(this, 'EventBus', 'default'),
      youtubeAppSecret,
      taskMonitoring,
    });

    new WebSocketAPIConstruct(this, 'WebSocketAPI', {
      userPool: userManagement.userPool,
      tasksTable: dataStore.tasksTable,
      userPoolClient: userManagement.userPoolClient,
      domainName,
    });

    new APIConstruct(this, 'API', {
      streamIngestionFunction: streamIngestion.stepFunction,
      renderJob: {
        jobQueue: batchEnvironment.cpuJobQueue,
        jobDefinition: renderJob.jobDefinition,
      },
      userPool: userManagement.userPool,
      userPoolClients: [userManagement.userPoolClient],
      openaiSecret,
      youtubeAppSecret,
      videoMetadataTable: dataStore.videoMetadataTable,
      streamsTable: dataStore.streamsTable,
      streamSeriesTable: dataStore.streamSeriesTable,
      episodesTable: dataStore.episodesTable,
      profilesTable: dataStore.profilesTable,
      tasksTable: dataStore.tasksTable,

      youtubeUploaderAPILambda: youtubeUploader.apiLambda,

      domainName,
    });
  }
}
