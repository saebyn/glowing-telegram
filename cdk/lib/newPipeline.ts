import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as batch from 'aws-cdk-lib/aws-batch';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as events from 'aws-cdk-lib/aws-events';
import * as targets from 'aws-cdk-lib/aws-events-targets';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import ServiceLambdaConstruct from './util/serviceLambda';

export interface NewPipelineConstructProps {
  vpc: ec2.IVpc;
  cpuJobQueue: batch.IJobQueue;
  inputBucket: s3.IBucket;
  outputBucket: s3.IBucket;
  mediaDomain: string;
  database: rds.IDatabaseCluster;
  databaseSecret: secretsmanager.ISecret;
  environmentName: string;
  tagOrDigest?: string;
}

/** Smallest deployable startRender pipeline backed by Streamosaic Postgres. */
export default class NewPipelineConstruct extends Construct {
  public readonly lambda: lambda.Function;
  public readonly statusHandler: lambda.Function;
  public readonly jobDefinition: batch.IJobDefinition;

  constructor(scope: Construct, id: string, props: NewPipelineConstructProps) {
    super(scope, id);

    props.vpc.addInterfaceEndpoint('BatchEndpoint', {
      service: ec2.InterfaceVpcEndpointAwsService.BATCH,
      subnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
    });
    props.vpc.addInterfaceEndpoint('SecretsManagerEndpoint', {
      service: ec2.InterfaceVpcEndpointAwsService.SECRETS_MANAGER,
      subnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
    });

    const jobExecutionRole = new iam.Role(this, 'JobExecutionRole', {
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
    props.inputBucket.grantRead(jobRole);
    props.outputBucket.grantWrite(jobRole);
    props.database.grantConnect(jobRole, 'postgres');
    props.databaseSecret.grantRead(jobRole);

    const renderRepository = ecr.Repository.fromRepositoryName(
      this,
      'RenderJobRepository',
      'glowing-telegram/render-job-new',
    );
    const logGroup = new logs.LogGroup(this, 'RenderLogGroup', {
      logGroupName: '/glowing-telegram/batch/render-job-new',
      retention: logs.RetentionDays.ONE_WEEK,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });
    const container = new batch.EcsFargateContainerDefinition(this, 'Container', {
      cpu: 16,
      memory: cdk.Size.gibibytes(32),
      ephemeralStorageSize: cdk.Size.gibibytes(100),
      assignPublicIp: true,
      executionRole: jobExecutionRole,
      jobRole,
      image: ecs.ContainerImage.fromEcrRepository(
        renderRepository,
        props.tagOrDigest ?? 'latest',
      ),
      command: ['Ref::render_job_id'],
      environment: {
        INPUT_BUCKET: props.inputBucket.bucketName,
        OUTPUT_BUCKET: props.outputBucket.bucketName,
        MEDIA_DOMAIN: props.mediaDomain,
        DATABASE_ENDPOINT: props.database.clusterEndpoint.hostname,
        DATABASE_PORT: props.database.clusterEndpoint.port.toString(),
        DATABASE_NAME: 'vectors',
        DATABASE_SECRET_ARN: props.databaseSecret.secretArn,
      },
      logging: ecs.LogDrivers.awsLogs({
        streamPrefix: 'render-job-new',
        logGroup,
      }),
    });
    this.jobDefinition = new batch.EcsJobDefinition(this, 'JobDefinition', {
      container,
      parameters: { render_job_id: '<render_job_id>' },
      retryAttempts: 1,
      timeout: cdk.Duration.hours(2),
    });

    const pipelineService = new ServiceLambdaConstruct(this, 'PipelineService', {
      name: 'pipeline',
      tagOrDigest: props.tagOrDigest,
      lambdaOptions: {
        functionName: `streamosaic-pipeline-${props.environmentName}`,
        description: 'Starts Streamosaic render jobs',
        timeout: cdk.Duration.seconds(30),
        vpc: props.vpc,
        vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
        environment: {
          DATABASE_ENDPOINT: props.database.clusterEndpoint.hostname,
          DATABASE_PORT: props.database.clusterEndpoint.port.toString(),
          DATABASE_NAME: 'vectors',
          DATABASE_SECRET_ARN: props.databaseSecret.secretArn,
          RENDER_JOB_QUEUE: props.cpuJobQueue.jobQueueArn,
          RENDER_JOB_DEFINITION: this.jobDefinition.jobDefinitionArn,
        },
      },
    });
    this.lambda = pipelineService.lambda;
    props.database.grantConnect(this.lambda, 'postgres');
    props.databaseSecret.grantRead(this.lambda);
    this.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['batch:SubmitJob'],
        resources: [
          props.cpuJobQueue.jobQueueArn,
          this.jobDefinition.jobDefinitionArn,
        ],
      }),
    );
    this.lambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['batch:TerminateJob'],
        resources: [
          cdk.Stack.of(this).formatArn({
            service: 'batch',
            resource: 'job',
            resourceName: '*',
          }),
        ],
      }),
    );

    const statusService = new ServiceLambdaConstruct(this, 'StatusService', {
      name: 'render-job-status-handler',
      tagOrDigest: props.tagOrDigest,
      lambdaOptions: {
        functionName: `streamosaic-render-status-${props.environmentName}`,
        description: 'Records Streamosaic render infrastructure failures',
        timeout: cdk.Duration.seconds(30),
        vpc: props.vpc,
        vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
        environment: {
          DATABASE_ENDPOINT: props.database.clusterEndpoint.hostname,
          DATABASE_PORT: props.database.clusterEndpoint.port.toString(),
          DATABASE_NAME: 'vectors',
          DATABASE_SECRET_ARN: props.databaseSecret.secretArn,
        },
      },
    });
    this.statusHandler = statusService.lambda;
    props.database.grantConnect(this.statusHandler, 'postgres');
    props.databaseSecret.grantRead(this.statusHandler);

    new events.Rule(this, 'RenderFailureRule', {
      eventPattern: {
        source: ['aws.batch'],
        detailType: ['Batch Job State Change'],
        detail: {
          jobDefinition: [this.jobDefinition.jobDefinitionArn],
          status: ['FAILED'],
        },
      },
      targets: [new targets.LambdaFunction(this.statusHandler)],
    });

    new cdk.CfnOutput(this, 'PipelineLambdaArn', {
      value: this.lambda.functionArn,
    });
  }
}
