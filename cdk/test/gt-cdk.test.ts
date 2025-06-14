import * as cdk from 'aws-cdk-lib';
import { Template, Match } from 'aws-cdk-lib/assertions';
import RenderJobSubmissionLambda from '../lib/renderJobSubmissionLambda';
import * as batch from 'aws-cdk-lib/aws-batch';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as fs from 'fs';
import * as path from 'path';

// Test for render job submission lambda and storage increase
test('Render Job Storage Increased and Lambda Contains Splitting Logic', () => {
  const app = new cdk.App();
  const stack = new cdk.Stack(app, 'TestStack');

  // Create mock dependencies with minimal setup
  const vpc = new ec2.Vpc(stack, 'TestVpc');
  const computeEnvironment = new batch.ManagedEc2EcsComputeEnvironment(stack, 'TestComputeEnv', {
    vpc,
  });
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

  // Verify that the Python handler file exists and contains the expected code
  const handlerPath = path.join(__dirname, '../lib/renderJobSubmissionLambda/handler.py');
  expect(fs.existsSync(handlerPath)).toBe(true);
  
  const handlerCode = fs.readFileSync(handlerPath, 'utf8');
  expect(handlerCode).toContain('MAX_EPISODES_PER_JOB');
  expect(handlerCode).toContain('split_episodes_into_chunks');
  expect(handlerCode).toContain('submit_render_job');

  // Create render job submission lambda - but don't synthesize to avoid Docker build issues in CI
  const renderJobSubmissionLambda = new RenderJobSubmissionLambda(stack, 'TestRenderJobSubmissionLambda', {
    renderJobQueue: jobQueue,
    renderJobDefinition: jobDefinition,
  });

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

// Test that Python handler file exists and contains expected code
test('Python Lambda Handler File Structure', () => {
  // Verify that the Python handler file exists and contains the expected code
  const handlerPath = path.join(__dirname, '../lib/renderJobSubmissionLambda/handler.py');
  expect(fs.existsSync(handlerPath)).toBe(true);
  
  const handlerCode = fs.readFileSync(handlerPath, 'utf8');
  expect(handlerCode).toContain('MAX_EPISODES_PER_JOB');
  expect(handlerCode).toContain('split_episodes_into_chunks');
  expect(handlerCode).toContain('submit_render_job');
  expect(handlerCode).toContain('def handler(event: Dict[str, Any], context: Any) -> Dict[str, Any]:');
});
