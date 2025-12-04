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
import EmbeddingServiceConstruct from './batch/embeddingServiceJob';
import TaskMonitoringConstruct from './taskMonitoring';
import MediaServeConstruct from './mediaServeConstruct';
import RenderJobConstruct from './batch/renderJob';
import YoutubeUploader from './youtubeUploader';
import WebSocketAPIConstruct from './websocketApi';
import TwitchChatProcessingConstruct from './twitchChatProcessing';
import { EventBus } from 'aws-cdk-lib/aws-events';

interface AppStackProps extends cdk.StackProps {
  domainName: string;
  imageVersion?: string;
}

const YOUTUBE_USER_SECRET_BASE_PATH = 'gt/youtube/user';
const TWITCH_USER_SECRET_BASE_PATH = 'gt/twitch/user';

export default class AppStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: AppStackProps) {
    const { domainName, imageVersion, ...restProps } = props;

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

    const dataStore = new DatastoreConstruct(this, 'Datastore', { vpc });

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
        modelCacheFileSystem: batchEnvironment.modelCacheFileSystem,
        modelCacheAccessPoint: batchEnvironment.modelCacheAccessPoint,
        imageVersion,
      },
    );

    const videoIngester = new VideoIngestorConstruct(this, 'VideoIngesterJob', {
      jobQueue: batchEnvironment.cpuJobQueue,
      outputBucket: dataStore.outputBucket,
      videoMetadataTable: dataStore.videoMetadataTable,
      videoArchiveBucket: dataStore.videoArchive,
      enableAutomaticIngestion: true,
      imageVersion,
    });

    const embeddingService = new EmbeddingServiceConstruct(this, 'EmbeddingService', {
      videoMetadataTable: dataStore.videoMetadataTable,
      vectorDatabase: dataStore.vectorDatabase,
      vectorDatabaseSecret: dataStore.vectorDatabaseSecret,
      openaiSecret,
      jobQueue: batchEnvironment.cpuJobQueue,
      imageVersion,
    });

    const mediaServe = new MediaServeConstruct(this, 'MediaServe', {
      mediaOutputBucket: dataStore.outputBucket,
      videoMetadataTable: dataStore.videoMetadataTable,
      domainName,
      imageVersion,
    });

    const taskMonitoring = new TaskMonitoringConstruct(this, 'TaskMonitoring', {
      tasksTable: dataStore.tasksTable,
    });

    const streamIngestion = new StreamIngestion(this, 'StreamIngestion', {
      audioTranscriberJob: audioTranscriber.jobDefinition,
      videoIngesterJob: videoIngester.jobDefinition,
      embeddingServiceJob: embeddingService.jobDefinition,
      cpuBatchJobQueue: batchEnvironment.cpuJobQueue,
      gpuBatchJobQueue: batchEnvironment.gpuJobQueue,
      videoMetadataTable: dataStore.videoMetadataTable,
      streamsTable: dataStore.streamsTable,
      videoArchive: dataStore.videoArchive,
      openaiSecret,
      mediaDistribution: mediaServe.distribution,
      taskMonitoring,
      imageVersion,
    });

    const renderJob = new RenderJobConstruct(this, 'RenderJob', {
      inputBucket: dataStore.videoArchive,
      outputBucket: dataStore.outputBucket,
      episodeTable: dataStore.episodesTable,
      jobQueue: batchEnvironment.cpuJobQueue,
      taskMonitoring,
      imageVersion,
    });

    const youtubeUploader = new YoutubeUploader(this, 'YoutubeUploader', {
      episodeTable: dataStore.episodesTable,
      jobQueue: batchEnvironment.cpuJobQueue,
      mediaOutputBucket: dataStore.outputBucket,
      eventBus: EventBus.fromEventBusName(this, 'EventBus', 'default'),
      youtubeAppSecret,
      youtubeUserSecretBasePath: YOUTUBE_USER_SECRET_BASE_PATH,
      taskMonitoring,
      imageVersion,
    });

    new WebSocketAPIConstruct(this, 'WebSocketAPI', {
      userPool: userManagement.userPool,
      tasksTable: dataStore.tasksTable,
      streamWidgetsTable: dataStore.streamWidgetsTable,
      userPoolClient: userManagement.userPoolClient,
      domainName,
    });

    const twitchChatProcessing = new TwitchChatProcessingConstruct(this, 'TwitchChatProcessing', {
      chatMessagesTable: dataStore.chatMessagesTable,
      imageVersion,
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
      youtubeUserSecretBasePath: YOUTUBE_USER_SECRET_BASE_PATH,
      twitchUserSecretBasePath: TWITCH_USER_SECRET_BASE_PATH,

      videoMetadataTable: dataStore.videoMetadataTable,
      streamsTable: dataStore.streamsTable,
      streamSeriesTable: dataStore.streamSeriesTable,
      episodesTable: dataStore.episodesTable,
      profilesTable: dataStore.profilesTable,
      tasksTable: dataStore.tasksTable,
      projectsTable: dataStore.projectsTable,
      chatMessagesTable: dataStore.chatMessagesTable,
      streamWidgetsTable: dataStore.streamWidgetsTable,
      chatQueue: twitchChatProcessing.chatQueue,

      youtubeUploaderAPILambda: youtubeUploader.apiLambda,

      domainName,
      imageVersion,
    });
  }
}
