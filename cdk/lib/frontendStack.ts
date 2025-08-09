import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deployment from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as path from 'path';

interface FrontendStackProps extends cdk.StackProps {
  // Keep frontendVersion for backwards compatibility, but it won't be used for origin path
  frontendVersion: string;
}

export default class FrontendStack extends cdk.Stack {
  public readonly assetBucket: s3.IBucket;
  public readonly domainName: string;
  public readonly versionSelectorFunction: lambda.Function;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    const { frontendVersion, ...restProps } = props;

    // Lambda@Edge functions must be deployed in us-east-1, but CDK handles this automatically
    // when used with CloudFront. The Lambda function will be replicated to edge locations.
    super(scope, id, { ...restProps, env: { ...restProps.env, region: 'us-east-1' } });

    this.assetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      publicReadAccess: false, // We'll add specific policy for version config
    });

    // Create Lambda@Edge function for dynamic version selection
    // Note: Lambda@Edge functions must be created in us-east-1 region
    this.versionSelectorFunction = new lambda.Function(this, 'VersionSelectorFunction', {
      runtime: lambda.Runtime.NODEJS_18_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset(path.join(__dirname, '../lambda/version-selector')),
      timeout: cdk.Duration.seconds(5),
      memorySize: 128,
      environment: {
        BUCKET_NAME: this.assetBucket.bucketName,
        FALLBACK_VERSION: frontendVersion, // Use original frontendVersion as fallback
      },
      // Lambda@Edge specific configuration
      role: new iam.Role(this, 'VersionSelectorRole', {
        assumedBy: new iam.CompositePrincipal(
          new iam.ServicePrincipal('lambda.amazonaws.com'),
          new iam.ServicePrincipal('edgelambda.amazonaws.com')
        ),
        managedPolicies: [
          iam.ManagedPolicy.fromAwsManagedPolicyName('service-role/AWSLambdaBasicExecutionRole'),
        ],
        inlinePolicies: {
          S3ReadAccess: new iam.PolicyDocument({
            statements: [
              new iam.PolicyStatement({
                effect: iam.Effect.ALLOW,
                actions: ['s3:GetObject'],
                resources: [`${this.assetBucket.bucketArn}/config/version.json`],
              }),
            ],
          }),
        },
      }),
    });

    // Add bucket policy to allow public read access to version config
    this.assetBucket.addToResourcePolicy(
      new iam.PolicyStatement({
        effect: iam.Effect.ALLOW,
        principals: [new iam.AnyPrincipal()],
        actions: ['s3:GetObject'],
        resources: [`${this.assetBucket.bucketArn}/config/version.json`],
      })
    );

    // Create CloudFront origin without hardcoded version path
    const origin = origins.S3BucketOrigin.withOriginAccessControl(
      this.assetBucket
      // No originPath - Lambda@Edge will handle version routing
    );

    const distribution = new cloudfront.Distribution(
      this,
      'FrontendDistribution',
      {
        comment: 'Frontend Distribution for Glowing-Telegram with Dynamic Version Selection',
        defaultRootObject: 'index.html',
        defaultBehavior: {
          viewerProtocolPolicy:
            cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
          origin,
          // Add Lambda@Edge function for viewer request
          edgeLambdas: [
            {
              functionVersion: this.versionSelectorFunction.currentVersion,
              eventType: cloudfront.LambdaEdgeEventType.VIEWER_REQUEST,
            },
          ],
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

    // Upload the default version config file
    new s3deployment.BucketDeployment(this, 'VersionConfigDeployment', {
      sources: [s3deployment.Source.asset(path.join(__dirname, '../config'))],
      destinationBucket: this.assetBucket,
      destinationKeyPrefix: 'config/',
    });
  }
}
