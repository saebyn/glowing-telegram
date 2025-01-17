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
  public readonly domainName: string;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    const { frontendVersion, ...restProps } = props;

    super(scope, id, restProps);

    this.assetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const origin = origins.S3BucketOrigin.withOriginAccessControl(
      this.assetBucket,
      {
        originPath: `/${frontendVersion}`,
      },
    );

    const distribution = new cloudfront.Distribution(
      this,
      'FrontendDistribution',
      {
        defaultRootObject: 'index.html',
        defaultBehavior: {
          viewerProtocolPolicy:
            cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
          origin,
        },
        errorResponses: [
          {
            httpStatus: 403,
            responseHttpStatus: 200,
            responsePagePath: '/index.html',
          },
        ],
      },
    );

    this.domainName = distribution.distributionDomainName;
  }
}
