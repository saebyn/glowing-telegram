import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';

export default class DatastoreConstruct extends Construct {
  public readonly videoArchive: s3.IBucket;
  public readonly outputBucket: s3.IBucket;
  public readonly vectorDatabase: rds.IDatabaseCluster;
  public readonly vectorDatabaseSecret: secretsmanager.ISecret;
  public readonly episodesTable: dynamodb.ITable;
  public readonly profilesTable: dynamodb.ITable;
  public readonly streamSeriesTable: dynamodb.ITable;
  public readonly streamsTable: dynamodb.ITable;
  public readonly videoMetadataTable: dynamodb.ITable;
  public readonly tasksTable: dynamodb.ITable;
  public readonly projectsTable: dynamodb.ITable;
  public readonly chatMessagesTable: dynamodb.ITable;
  public readonly streamWidgetsTable: dynamodb.ITable;

  constructor(scope: Construct, id: string, props: { vpc: ec2.IVpc }) {
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

    // Create Aurora Serverless v2 cluster for vector storage
    const vectorDatabaseSecret = new secretsmanager.Secret(this, 'VectorDatabaseSecret', {
      description: 'Aurora Serverless v2 cluster credentials for vector storage',
      generateSecretString: {
        secretStringTemplate: JSON.stringify({ username: 'postgres' }),
        generateStringKey: 'password',
        excludeCharacters: '"@/\\',
      },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    this.vectorDatabaseSecret = vectorDatabaseSecret;

    const subnetGroup = new rds.SubnetGroup(this, 'VectorDatabaseSubnetGroup', {
      description: 'Subnet group for vector database',
      vpc: props.vpc,
      vpcSubnets: {
        subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS,
      },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const securityGroup = new ec2.SecurityGroup(this, 'VectorDatabaseSecurityGroup', {
      vpc: props.vpc,
      description: 'Security group for vector database',
    });

    // Allow connections from VPC CIDR on PostgreSQL port
    securityGroup.addIngressRule(
      ec2.Peer.ipv4(props.vpc.vpcCidrBlock),
      ec2.Port.tcp(5432),
      'Allow PostgreSQL connections from VPC'
    );

    const vectorDatabase = new rds.DatabaseCluster(this, 'VectorDatabase', {
      engine: rds.DatabaseClusterEngine.auroraPostgres({
        version: rds.AuroraPostgresEngineVersion.VER_16_1,
      }),
      writer: rds.ClusterInstance.serverlessV2('writer', {
        scaleWithWriter: true,
      }),
      serverlessV2MinCapacity: 0.5,
      serverlessV2MaxCapacity: 4,
      credentials: rds.Credentials.fromSecret(vectorDatabaseSecret),
      defaultDatabaseName: 'vectors',
      vpc: props.vpc,
      subnetGroup,
      securityGroups: [securityGroup],
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      deletionProtection: true,
      storageEncrypted: true,
    });

    this.vectorDatabase = vectorDatabase;

    const episodesTable = new dynamodb.Table(this, 'EpisodesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
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
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
    });

    this.streamSeriesTable = new dynamodb.Table(this, 'StreamSeriesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
    });

    this.streamsTable = new dynamodb.Table(this, 'StreamsTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
    });

    const videoMetadataTable = new dynamodb.Table(this, 'VideoMetadataTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'key', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
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
      // no backups for ephemeral tasks table
    });

    this.projectsTable = new dynamodb.Table(this, 'ProjectsTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
    });

    const chatMessagesTable = new dynamodb.Table(this, 'ChatMessagesTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'user_id', type: dynamodb.AttributeType.STRING },
      sortKey: { name: 'timestamp', type: dynamodb.AttributeType.STRING },
      timeToLiveAttribute: 'ttl',
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
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

    // Create stream_widgets table
    const streamWidgetsTable = new dynamodb.Table(this, 'StreamWidgetsTable', {
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      stream: dynamodb.StreamViewType.NEW_AND_OLD_IMAGES,
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
    });

    // Add GSI for user_id (primary query pattern: get all widgets for a user)
    streamWidgetsTable.addGlobalSecondaryIndex({
      indexName: 'user_id-index',
      partitionKey: {
        name: 'user_id',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // Add GSI for access_token (for WebSocket authentication)
    streamWidgetsTable.addGlobalSecondaryIndex({
      indexName: 'access_token-index',
      partitionKey: {
        name: 'access_token',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // Add GSI for querying widgets by type (for scheduled updates)
    // Note: active is a boolean and cannot be used as a sort key, so we filter in the query
    streamWidgetsTable.addGlobalSecondaryIndex({
      indexName: 'type-index',
      partitionKey: {
        name: 'type',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    this.streamWidgetsTable = streamWidgetsTable;
  }
}
