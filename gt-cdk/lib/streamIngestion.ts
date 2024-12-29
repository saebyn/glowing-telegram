import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import type * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';

interface StreamIngestionConstructProps {
  audioTranscriberJob: batch.IJobDefinition;
  videoIngesterJob: batch.IJobDefinition;

  cpuBatchJobQueue: batch.IJobQueue;
  gpuBatchJobQueue: batch.IJobQueue;

  videoMetadataTable: dynamodb.ITable;
  streamsTable: dynamodb.ITable;

  videoArchive: s3.IBucket;

  openaiSecret: secretsmanager.ISecret;
}

export default class StreamIngestionConstruct extends Construct {
  constructor(
    scope: Construct,
    id: string,
    props: StreamIngestionConstructProps,
  ) {
    super(scope, id);

    // TODO
  }
}
