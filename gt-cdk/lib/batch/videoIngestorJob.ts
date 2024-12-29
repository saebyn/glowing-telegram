import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import type * as batch from 'aws-cdk-lib/aws-batch';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import type * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';

interface XConstructProps {}

/**
 * Video ingestor job construct for AWS Batch
 */
export default class XConstruct extends Construct {
  constructor(scope: Construct, id: string, props: XConstructProps) {
    super(scope, id);

    // TODO
  }
}
