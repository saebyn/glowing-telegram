import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deployment from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as path from 'node:path';
import * as fs from 'node:fs';

interface FrontendStackProps extends cdk.StackProps {
  // Frontend version used as fallback if config file is not available
  frontendVersion: string;
}

export default class FrontendStack extends cdk.Stack {
  public readonly assetBucket: s3.IBucket;
  public readonly domainName: string;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    const { frontendVersion, ...restProps } = props;

    super(scope, id, restProps);

    // Read current version from config file
    const currentVersion = this.getCurrentVersion(frontendVersion);

    this.assetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ACLS_ONLY,
      autoDeleteObjects: false, // Prevent accidental deletion of assets
    });

    // Create CloudFront origin with version-specific path
    const origin = origins.S3BucketOrigin.withOriginAccessControl(
      this.assetBucket,
      {
        originPath: `/${currentVersion}`, // Set origin path to version folder
      }
    );

    const distribution = new cloudfront.Distribution(
      this,
      'FrontendDistribution',
      {
        comment:
          'Frontend Distribution for Glowing-Telegram with Static Version Path',
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

    // Upload the default version config file
    new s3deployment.BucketDeployment(this, 'VersionConfigDeployment', {
      sources: [s3deployment.Source.asset(path.join(__dirname, '../config'))],
      destinationBucket: this.assetBucket,
      destinationKeyPrefix: 'config/',
    });
  }

  /**
   * Read the current version from the version config file, falling back to the provided version
   */
  private getCurrentVersion(fallbackVersion: string): string {
    try {
      const configPath = path.join(__dirname, '../config/version.json');
      if (fs.existsSync(configPath)) {
        const configContent = fs.readFileSync(configPath, 'utf-8');
        const config = JSON.parse(configContent);
        
        if (config.version && typeof config.version === 'string') {
          console.log(`Using version from config: ${config.version}`);
          return config.version;
        }
      }
    } catch (error) {
      console.warn(`Could not read version config: ${error}`);
    }
    
    console.log(`Using fallback version: ${fallbackVersion}`);
    return fallbackVersion;
  }
}
