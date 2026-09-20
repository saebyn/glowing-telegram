import * as cdk from 'aws-cdk-lib';
import { Match, Template } from 'aws-cdk-lib/assertions';
import * as batch from 'aws-cdk-lib/aws-batch';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import NewPipelineConstruct from '../lib/newPipeline';

function synthesizePipeline(): Template {
  const stack = new cdk.Stack(new cdk.App(), 'NewPipelineTest');
  const vpc = new ec2.Vpc(stack, 'Vpc', {
    natGateways: 0,
    maxAzs: 2,
    subnetConfiguration: [
      { name: 'Public', subnetType: ec2.SubnetType.PUBLIC, cidrMask: 24 },
      {
        name: 'Private',
        subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS,
        cidrMask: 24,
      },
    ],
  });
  const queue = batch.JobQueue.fromJobQueueArn(
    stack,
    'Queue',
    stack.formatArn({ service: 'batch', resource: 'job-queue', resourceName: 'cpu' }),
  );
  const inputBucket = s3.Bucket.fromBucketName(stack, 'Input', 'pipeline-input');
  const outputBucket = s3.Bucket.fromBucketName(stack, 'Output', 'pipeline-output');
  const secret = secretsmanager.Secret.fromSecretCompleteArn(
    stack,
    'DatabaseSecret',
    stack.formatArn({
      service: 'secretsmanager',
      resource: 'secret',
      resourceName: 'streamosaic-db-AbCdEf',
    }),
  );
  const database = rds.DatabaseCluster.fromDatabaseClusterAttributes(
    stack,
    'Database',
    {
      clusterIdentifier: 'gt-vectors',
      clusterResourceIdentifier: 'cluster-resource-id',
      clusterEndpointAddress: 'vectors.cluster.example.com',
      port: 5432,
      readerEndpointAddress: 'vectors-ro.cluster.example.com',
      securityGroups: [],
    },
  );

  new NewPipelineConstruct(stack, 'Pipeline', {
    vpc,
    cpuJobQueue: queue,
    inputBucket,
    outputBucket,
    mediaDomain: 'media.example.com',
    database,
    databaseSecret: secret,
    environmentName: 'test',
    tagOrDigest: 'test-tag',
  });

  return Template.fromStack(stack);
}

test('creates the startRender job definition and pipeline Lambda', () => {
  const template = synthesizePipeline();

  template.hasResourceProperties('AWS::Batch::JobDefinition', {
    Type: 'container',
    Parameters: { render_job_id: '<render_job_id>' },
    ContainerProperties: Match.objectLike({
      Command: ['Ref::render_job_id'],
      Image: Match.anyValue(),
      ResourceRequirements: Match.arrayWith([
        { Type: 'MEMORY', Value: '32768' },
        { Type: 'VCPU', Value: '16' },
      ]),
      EphemeralStorage: { SizeInGiB: 100 },
      NetworkConfiguration: { AssignPublicIp: 'ENABLED' },
      Environment: Match.arrayWith([
        { Name: 'INPUT_BUCKET', Value: 'pipeline-input' },
        { Name: 'OUTPUT_BUCKET', Value: 'pipeline-output' },
        { Name: 'MEDIA_DOMAIN', Value: 'media.example.com' },
        { Name: 'DATABASE_ENDPOINT', Value: 'vectors.cluster.example.com' },
        { Name: 'DATABASE_PORT', Value: '5432' },
        { Name: 'DATABASE_NAME', Value: 'vectors' },
        { Name: 'DATABASE_SECRET_ARN', Value: Match.anyValue() },
      ]),
    }),
  });
  template.hasResourceProperties('AWS::Lambda::Function', {
    FunctionName: 'streamosaic-pipeline-test',
    PackageType: 'Image',
    VpcConfig: Match.objectLike({ SubnetIds: Match.anyValue() }),
    Environment: {
      Variables: Match.objectLike({
        DATABASE_ENDPOINT: 'vectors.cluster.example.com',
        DATABASE_PORT: '5432',
        DATABASE_NAME: 'vectors',
        DATABASE_SECRET_ARN: Match.anyValue(),
        RENDER_JOB_QUEUE: Match.anyValue(),
        RENDER_JOB_DEFINITION: Match.anyValue(),
      }),
    },
  });
  template.hasResourceProperties('AWS::Lambda::Function', {
    FunctionName: 'streamosaic-render-status-test',
    PackageType: 'Image',
    Environment: {
      Variables: Match.objectLike({
        DATABASE_ENDPOINT: 'vectors.cluster.example.com',
        DATABASE_NAME: 'vectors',
      }),
    },
  });
  template.hasResourceProperties('AWS::Events::Rule', {
    EventPattern: Match.objectLike({
      source: ['aws.batch'],
      detail: Match.objectLike({ status: ['FAILED'] }),
    }),
    Targets: Match.anyValue(),
  });
  expect(JSON.stringify(template.toJSON())).toContain(
    'glowing-telegram/render-job-new:test-tag',
  );
  const outputs = template.toJSON().Outputs ?? {};
  expect(Object.keys(outputs)).toHaveLength(1);
  expect(Object.keys(outputs)[0]).toContain('PipelineLambdaArn');
  expect(Object.values(outputs)).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        Value: expect.objectContaining({ 'Fn::GetAtt': expect.anything() }),
      }),
    ]),
  );
});

test('grants only required storage, secret, and scoped Batch access', () => {
  const template = synthesizePipeline();
  const serialized = JSON.stringify(template.toJSON());

  expect(serialized).not.toContain('dynamodb:');
  expect(serialized).not.toContain('DYNAMODB_');
  template.hasResourceProperties('AWS::IAM::Policy', {
    PolicyDocument: {
      Statement: Match.arrayWith([
        Match.objectLike({ Action: 'batch:SubmitJob', Resource: Match.anyValue() }),
        Match.objectLike({
          Action: 'batch:TerminateJob',
          Resource: Match.anyValue(),
        }),
      ]),
    },
  });
  expect(serialized).toContain('s3:GetObject');
  expect(serialized).toContain('s3:PutObject');
  expect(serialized).toContain('secretsmanager:GetSecretValue');
  expect(serialized).toContain('job/*');
});

test('creates private Batch and Secrets Manager endpoints', () => {
  const template = synthesizePipeline();

  template.resourceCountIs('AWS::EC2::VPCEndpoint', 2);
  template.hasResourceProperties('AWS::EC2::VPCEndpoint', {
    VpcEndpointType: 'Interface',
    ServiceName: Match.anyValue(),
    PrivateDnsEnabled: true,
  });
  const endpoints = JSON.stringify(template.findResources('AWS::EC2::VPCEndpoint'));
  expect(endpoints).toContain('batch');
  expect(endpoints).toContain('secretsmanager');
});
