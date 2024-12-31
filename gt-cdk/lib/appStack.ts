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

export default class GtCdkStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

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

    // TODO import the resource from the existing Pulumi stack
    const openaiSecret = secretsmanager.Secret.fromSecretNameV2(
      this,
      'OpenaiSecret',
      'openai-secret',
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

    const videoIngester = new AudioTranscriberJobConstruct(
      this,
      'VideoIngesterJob',
      {
        outputBucket: dataStore.outputBucket,
        videoMetadataTable: dataStore.videoMetadataTable,
      },
    );

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
    });
  }
}
