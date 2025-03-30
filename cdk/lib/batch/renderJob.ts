import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as events from 'aws-cdk-lib/aws-events';
import type TaskMonitoringConstruct from '../taskMonitoring';

interface RenderJobConstructProps {
  inputBucket: s3.IBucket;
  outputBucket: s3.IBucket;
  episodeTable: dynamodb.ITable;
  jobQueue: batch.IJobQueue;
  taskMonitoring: TaskMonitoringConstruct;
}

/**
 * Render job construct for AWS Batch
 */
export default class RenderJobConstruct extends Construct {
  jobDefinition: batch.IJobDefinition;

  constructor(scope: Construct, id: string, props: RenderJobConstructProps) {
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

    props.inputBucket.grantRead(jobRole);
    props.outputBucket.grantWrite(jobRole);
    props.episodeTable.grantReadWriteData(jobRole);

    const repo = ecr.Repository.fromRepositoryName(
      this,
      'RenderJobRepository',
      'glowing-telegram/render-job',
    );

    const containerDefinition = new batch.EcsFargateContainerDefinition(
      this,
      'RenderJobContainerDefinition',
      {
        cpu: 16,
        memory: cdk.Size.gibibytes(32),
        assignPublicIp: true,
        jobRole,
        executionRole,
        command: ['Ref::record_ids'],
        image: ecs.ContainerImage.fromEcrRepository(repo, 'latest'),
        environment: {
          INPUT_BUCKET: props.inputBucket.bucketName,
          OUTPUT_BUCKET: props.outputBucket.bucketName,
          DYNAMODB_TABLE: props.episodeTable.tableName,
        },
        ephemeralStorageSize: cdk.Size.gibibytes(50),
      },
    );

    this.jobDefinition = new batch.EcsJobDefinition(
      this,
      'RenderJobDefinition',
      {
        container: containerDefinition,
        timeout: cdk.Duration.hours(2),
        parameters: {
          record_ids: '<record_ids>',
          user_id: '<user_id>',
        },
        retryAttempts: 1,
      },
    );

    new events.Rule(this, 'RenderJobRule', {
      eventPattern: {
        source: ['aws.batch'],
        detailType: ['Batch Job State Change'],
        detail: {
          jobName: [this.jobDefinition.jobDefinitionName],
          status: ['SUBMITTED', 'PENDING', 'RUNNABLE', 'STARTING'],
        },
      },
    }).addTarget(
      props.taskMonitoring.newEventTarget({
        name: events.EventField.fromPath('$.detail.jobName'),
        status: 'PENDING',
        time: events.EventField.fromPath('$.time'),
        record_id: events.EventField.fromPath('$.detail.parameters.record_ids'),
        task_type: 'render',
        user_id: events.EventField.fromPath('$.detail.parameters.user_id'),
      }),
    );

    new events.Rule(this, 'RenderJobCompleteRule', {
      eventPattern: {
        source: ['aws.batch'],
        detailType: ['Batch Job State Change'],
        detail: {
          jobName: [this.jobDefinition.jobDefinitionName],
          status: ['SUCCEEDED', 'FAILED', 'RUNNING'],
        },
      },
    }).addTarget(
      props.taskMonitoring.newEventTarget({
        name: events.EventField.fromPath('$.detail.jobName'),
        status: events.EventField.fromPath('$.detail.status'),
        time: events.EventField.fromPath('$.time'),
        record_id: events.EventField.fromPath('$.detail.parameters.record_ids'),
        user_id: events.EventField.fromPath('$.detail.parameters.user_id'),
        task_type: 'render',
      }),
    );
  }
}
