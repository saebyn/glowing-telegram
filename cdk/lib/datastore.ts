import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';

export default class DatastoreConstruct extends Construct {
  public readonly videoArchive: s3.IBucket;
  public readonly outputBucket: s3.IBucket;
  public readonly episodesTable: dynamodb.ITable;
  public readonly profilesTable: dynamodb.ITable;
  public readonly streamSeriesTable: dynamodb.ITable;
  public readonly streamsTable: dynamodb.ITable;
  public readonly videoMetadataTable: dynamodb.ITable;
  public readonly tasksTable: dynamodb.ITable;
  public readonly projectsTable: dynamodb.ITable;
  public readonly chatMessagesTable: dynamodb.ITable;

  constructor(scope: Construct, id: string) {
    super(scope, id);

    this.videoArchive = new s3.Bucket(this, 'VideoArchive', {
      versioned: true,
      encryption: s3.BucketEncryption.S3_MANAGED,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      eventBridgeEnabled: true,
      lifecycleRules: [
        {
          id: 'delete_old_markers',
          abortIncompleteMultipartUploadAfter: cdk.Duration.days(1),
        },
        {
          id: 'glacier archive',
          tagFilters: { Archive: 'true' },
          transitions: [
            {
              storageClass: s3.StorageClass.INFREQUENT_ACCESS,
              transitionAfter: cdk.Duration.days(30),
            },
            {
              storageClass: s3.StorageClass.GLACIER_INSTANT_RETRIEVAL,
              transitionAfter: cdk.Duration.days(60),
            },
            {
              storageClass: s3.StorageClass.GLACIER,
              transitionAfter: cdk.Duration.days(150),
            },
          ],
        },
      ],
      objectOwnership: s3.ObjectOwnership.BUCKET_OWNER_ENFORCED,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    this.outputBucket = new s3.Bucket(this, 'OutputBucket', {
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const episodesTable = new dynamodb.Table(this, 'EpisodesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    episodesTable.addGlobalSecondaryIndex({
      indexName: 'upload_status-upload_queue_timestamp-index',
      partitionKey: {
        name: 'upload_status',
        type: dynamodb.AttributeType.STRING,
      },
      sortKey: {
        name: 'upload_queue_timestamp',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.KEYS_ONLY,
    });

    this.episodesTable = episodesTable;

    this.profilesTable = new dynamodb.Table(this, 'ProfilesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    this.streamSeriesTable = new dynamodb.Table(this, 'StreamSeriesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    this.streamsTable = new dynamodb.Table(this, 'StreamsTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const videoMetadataTable = new dynamodb.Table(this, 'VideoMetadataTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'key', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    videoMetadataTable.addGlobalSecondaryIndex({
      indexName: 'stream_id-index',
      partitionKey: {
        name: 'stream_id',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    this.videoMetadataTable = videoMetadataTable;

    this.tasksTable = new dynamodb.Table(this, 'TasksTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      timeToLiveAttribute: 'ttl',
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      stream: dynamodb.StreamViewType.NEW_AND_OLD_IMAGES,
    });

    this.projectsTable = new dynamodb.Table(this, 'ProjectsTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const chatMessagesTable = new dynamodb.Table(this, 'ChatMessagesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'user_id', type: dynamodb.AttributeType.STRING },
      sortKey: { name: 'timestamp', type: dynamodb.AttributeType.STRING },
      timeToLiveAttribute: 'ttl',
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // Add GSI for querying by channel_id
    chatMessagesTable.addGlobalSecondaryIndex({
      indexName: 'channel_id-timestamp-index',
      partitionKey: {
        name: 'channel_id',
        type: dynamodb.AttributeType.STRING,
      },
      sortKey: {
        name: 'timestamp',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    this.chatMessagesTable = chatMessagesTable;
  }
}
