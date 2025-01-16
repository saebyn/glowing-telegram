import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';

interface FrontendStackProps extends cdk.StackProps {
  frontendVersion: string;
}

export default class FrontendStack extends cdk.Stack {
  public readonly assetBucket: s3.IBucket;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    const { frontendVersion, ...restProps } = props;

    super(scope, id, restProps);

    this.assetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    new cloudfront.Distribution(this, 'FrontendDistribution', {
      defaultRootObject: 'index.html',
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withOriginAccessControl(
          this.assetBucket,
          {
            originPath: `/${frontendVersion}`,
          },
        ),
      },
    });
  }
}
