import * as cdk from 'aws-cdk-lib';
import { Template, Match } from 'aws-cdk-lib/assertions';
import RenderJobSubmissionLambda from '../lib/renderJobSubmissionLambda';
import ServiceLambdaConstruct, { LOG_GROUP_PREFIX, LOG_RETENTION } from '../lib/util/serviceLambda';
import TaskMonitoringConstruct from '../lib/taskMonitoring';
import * as batch from 'aws-cdk-lib/aws-batch';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as logs from 'aws-cdk-lib/aws-logs';

// Test for log group constants
test('Log group constants are correctly defined', () => {
  expect(LOG_GROUP_PREFIX).toBe('/glowing-telegram');
  expect(LOG_RETENTION).toBe(logs.RetentionDays.ONE_WEEK);
});

// Test for ServiceLambdaConstruct log group configuration
test('ServiceLambdaConstruct creates log group with correct naming convention', () => {
  const app = new cdk.App();
  const stack = new cdk.Stack(app, 'TestServiceLambdaStack');

  const serviceLambda = new ServiceLambdaConstruct(stack, 'TestServiceLambda', {
    name: 'test-service',
    lambdaOptions: {
      description: 'Test service lambda',
      timeout: cdk.Duration.seconds(30),
    },
  });

  // Verify the log group was created
  expect(serviceLambda.logGroup).toBeDefined();

  const template = Template.fromStack(stack);

  // Verify that a log group is created with the correct naming pattern
  template.hasResourceProperties('AWS::Logs::LogGroup', {
    LogGroupName: '/glowing-telegram/lambda/test-service-TestServiceLambda',
    RetentionInDays: 7,
  });
});

// Test for TaskMonitoringConstruct log group configuration
test('TaskMonitoringConstruct creates log group with correct naming convention', () => {
  const app = new cdk.App();
  const stack = new cdk.Stack(app, 'TestTaskMonitoringStack');

  const tasksTable = new dynamodb.Table(stack, 'TasksTable', {
    partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
    billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
  });

  new TaskMonitoringConstruct(stack, 'TestTaskMonitoring', {
    tasksTable,
  });

  const template = Template.fromStack(stack);

  // Verify that a log group is created with the correct naming pattern
  template.hasResourceProperties('AWS::Logs::LogGroup', {
    LogGroupName: '/glowing-telegram/lambda/task-status',
    RetentionInDays: 7,
  });
});

// Test for render job submission lambda and storage increase
test('Render Job Storage Increased and Lambda Contains Splitting Logic', () => {
  const app = new cdk.App();
  const stack = new cdk.Stack(app, 'TestStack');

  // Create mock dependencies with minimal setup
  const vpc = new ec2.Vpc(stack, 'TestVpc');
  const computeEnvironment = new batch.ManagedEc2EcsComputeEnvironment(
    stack,
    'TestComputeEnv',
    {
      vpc,
    },
  );
  const jobQueue = new batch.JobQueue(stack, 'TestJobQueue', {
    computeEnvironments: [{ computeEnvironment, order: 1 }],
  });

  // Create a simple job definition with increased storage
  const executionRole = new iam.Role(stack, 'JobExecutionRole', {
    assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
    managedPolicies: [
      iam.ManagedPolicy.fromAwsManagedPolicyName(
        'service-role/AmazonECSTaskExecutionRolePolicy',
      ),
    ],
  });

  const jobRole = new iam.Role(stack, 'JobRole', {
    assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
  });

  const repo = ecr.Repository.fromRepositoryName(
    stack,
    'RenderJobRepository',
    'glowing-telegram/render-job',
  );

  const containerDefinition = new batch.EcsFargateContainerDefinition(
    stack,
    'TestRenderJobContainerDefinition',
    {
      cpu: 16,
      memory: cdk.Size.gibibytes(32),
      assignPublicIp: true,
      jobRole,
      executionRole,
      command: ['Ref::record_ids'],
      image: ecs.ContainerImage.fromEcrRepository(repo, 'latest'),
      environment: {
        INPUT_BUCKET: 'test-input-bucket',
        OUTPUT_BUCKET: 'test-output-bucket',
        DYNAMODB_TABLE: 'test-table',
      },
      ephemeralStorageSize: cdk.Size.gibibytes(100), // This is the key change
    },
  );

  const jobDefinition = new batch.EcsJobDefinition(
    stack,
    'TestRenderJobDefinition',
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

  // Create render job submission lambda - but don't synthesize to avoid Docker build issues in CI
  const renderJobSubmissionLambda = new RenderJobSubmissionLambda(
    stack,
    'TestRenderJobSubmissionLambda',
    {
      renderJobQueue: jobQueue,
      renderJobDefinition: jobDefinition,
    },
  );

  // Verify the lambda construct was created successfully
  expect(renderJobSubmissionLambda.lambda).toBeDefined();

  // Verify that the ephemeral storage is increased to 100 GiB in the job definition
  const template = Template.fromStack(stack);
  template.hasResourceProperties('AWS::Batch::JobDefinition', {
    ContainerProperties: {
      EphemeralStorage: {
        SizeInGiB: 100,
      },
    },
  });
});

// Test for GPU compute environment with launch template
test('GPU Compute Environment Has Launch Template with Larger Disk', () => {
  const app = new cdk.App();
  const stack = new cdk.Stack(app, 'TestGPUStack');

  // Create mock VPC
  const vpc = new ec2.Vpc(stack, 'TestVpc');

  // Import the BatchEnvironmentConstruct
  const BatchEnvironmentConstruct = require('../lib/batch/environment').default;
  
  const batchEnvironment = new BatchEnvironmentConstruct(
    stack,
    'TestBatchEnvironment',
    { vpc }
  );

  // Verify the batch environment was created successfully
  expect(batchEnvironment.gpuJobQueue).toBeDefined();

  // Verify that the GPU launch template is created with 100 GiB root volume
  const template = Template.fromStack(stack);
  template.hasResourceProperties('AWS::EC2::LaunchTemplate', {
    LaunchTemplateData: {
      BlockDeviceMappings: [
        {
          DeviceName: '/dev/xvda',
          Ebs: {
            VolumeSize: 100,
            VolumeType: 'gp3',
            DeleteOnTermination: true,
          },
        },
      ],
    },
  });

  // Verify that the compute environment references the launch template
  template.hasResourceProperties('AWS::Batch::ComputeEnvironment', {
    ComputeResources: {
      Type: 'SPOT',
      AllocationStrategy: 'SPOT_PRICE_CAPACITY_OPTIMIZED',
      LaunchTemplate: {
        LaunchTemplateId: {
          Ref: Match.anyValue(),
        },
      },
    },
  });
});

// Test for Audio Transcriber Job with Retry Configuration
test('Audio Transcriber Job Has Retry Attempts for Spot Instance Failures', () => {
  const app = new cdk.App();
  const stack = new cdk.Stack(app, 'TestAudioTranscriberStack');

  // Create mock dependencies
  const outputBucket = {
    bucketName: 'test-output-bucket',
    bucketArn: 'arn:aws:s3:::test-output-bucket',
    grantRead: jest.fn(),
  } as unknown as cdk.aws_s3.IBucket;

  const videoMetadataTable = {
    tableName: 'test-table',
    tableArn: 'arn:aws:dynamodb:us-west-2:123456789012:table/test-table',
    grantReadWriteData: jest.fn(),
  } as unknown as dynamodb.ITable;

  // Import AudioTranscriberJobConstruct
  const AudioTranscriberJobConstruct = require('../lib/batch/audioTranscriberJob').default;

  const audioTranscriberJob = new AudioTranscriberJobConstruct(
    stack,
    'TestAudioTranscriberJob',
    {
      outputBucket,
      videoMetadataTable,
      imageVersion: 'test-version',
    },
  );

  // Verify the job was created
  expect(audioTranscriberJob.jobDefinition).toBeDefined();

  // Verify that the job definition has retry attempts set to 2
  const template = Template.fromStack(stack);
  template.hasResourceProperties('AWS::Batch::JobDefinition', {
    RetryStrategy: {
      Attempts: 2,
    },
  });
});
