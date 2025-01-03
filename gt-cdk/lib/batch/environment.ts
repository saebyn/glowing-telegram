import type * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as batch from 'aws-cdk-lib/aws-batch';
import * as ec2 from 'aws-cdk-lib/aws-ec2';

interface BatchEnvironmentConstructProps {
  vpc: ec2.IVpc;
}

/**
 * AWS Batch environment construct
 */
export default class BatchEnvironmentConstruct extends Construct {
  cpuJobQueue: batch.IJobQueue;
  gpuJobQueue: cdk.aws_batch.JobQueue;

  constructor(
    scope: Construct,
    id: string,
    props: BatchEnvironmentConstructProps,
  ) {
    super(scope, id);

    const vpc = props.vpc;

    const sg = new ec2.SecurityGroup(this, 'SecurityGroup', {
      vpc,
      allowAllOutbound: true,
    });

    const fargateComputeEnvironment = new batch.FargateComputeEnvironment(
      this,
      'ComputeEnvironment',
      {
        vpc,
        securityGroups: [sg],
        vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
        maxvCpus: 16,
      },
    );

    const spotComputeEnvironment = new batch.ManagedEc2EcsComputeEnvironment(
      this,
      'SpotComputeEnvironment',
      {
        vpc,
        securityGroups: [sg],
        vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
        minvCpus: 0,
        maxvCpus: 16,
        instanceTypes: [new ec2.InstanceType('g4dn')],
        spot: true,
        allocationStrategy:
          batch.AllocationStrategy.SPOT_PRICE_CAPACITY_OPTIMIZED,
      },
    );

    this.cpuJobQueue = new batch.JobQueue(this, 'JobQueue', {
      computeEnvironments: [
        {
          computeEnvironment: fargateComputeEnvironment,
          order: 1,
        },
      ],
    });

    this.gpuJobQueue = new batch.JobQueue(this, 'GPUJobQueue', {
      computeEnvironments: [
        {
          computeEnvironment: spotComputeEnvironment,
          order: 1,
        },
      ],
    });
  }
}
