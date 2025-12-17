import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as rds from 'aws-cdk-lib/aws-rds';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as logs from 'aws-cdk-lib/aws-logs';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from '../util/serviceLambda';

interface EmbeddingServiceConstructProps {
  videoMetadataTable: dynamodb.ITable;
  vectorDatabase: rds.IDatabaseCluster;
  vectorDatabaseSecret: secretsmanager.ISecret;
  openaiSecret: secretsmanager.ISecret;

  jobQueue: batch.IJobQueue;
  tagOrDigest?: string;
}

/**
 * Embedding service job construct for AWS Batch
 */
export default class EmbeddingServiceConstruct extends Construct {
  jobDefinition: batch.IJobDefinition;

  constructor(
    scope: Construct,
    id: string,
    props: EmbeddingServiceConstructProps,
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

    props.videoMetadataTable.grantReadData(jobRole);
    props.vectorDatabase.grantConnect(jobRole, 'postgres');
    props.vectorDatabaseSecret.grantRead(jobRole);
    props.openaiSecret.grantRead(jobRole);

    const repo = ecr.Repository.fromRepositoryName(
      this,
      'EmbeddingServiceJobRepository',
      'glowing-telegram/embedding-service',
    );

    // Create log group for embedding service batch job
    const logGroup = new logs.LogGroup(this, 'LogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/batch/embedding-service`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    const containerDefinition = new batch.EcsFargateContainerDefinition(
      this,
      'JobContainerDefinition',
      {
        cpu: 1,
        memory: cdk.Size.gibibytes(2),
        assignPublicIp: true,
        jobRole,
        executionRole,
        command: ['process', 'Ref::video_key'],
        image: ecs.ContainerImage.fromEcrRepository(repo, props.tagOrDigest || 'latest'),
        environment: {
          DYNAMODB_TABLE: props.videoMetadataTable.tableName,
          DATABASE_SECRET_ARN: props.vectorDatabaseSecret.secretArn,
          DATABASE_ENDPOINT: props.vectorDatabase.clusterEndpoint.hostname,
          DATABASE_PORT: '5432',
          DATABASE_NAME: 'vectors',
          OPENAI_SECRET_ARN: props.openaiSecret.secretArn,
          OPENAI_MODEL: 'text-embedding-3-small',
        },
        logging: ecs.LogDrivers.awsLogs({
          streamPrefix: 'embedding-service',
          logGroup,
        }),
      },
    );

    this.jobDefinition = new batch.EcsJobDefinition(this, 'JobDefinition', {
      container: containerDefinition,
      timeout: cdk.Duration.minutes(15),
      parameters: {
        video_key: '<video_key>',
      },
      retryAttempts: 2,
    });
  }
}