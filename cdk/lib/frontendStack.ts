import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deployment from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as s3n from 'aws-cdk-lib/aws-s3-notifications';
import * as path from 'node:path';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from './util/serviceLambda';

interface FrontendStackProps extends cdk.StackProps {
  // Frontend version used as fallback if config file is not available
  frontendVersion: string;
}

export default class FrontendStack extends cdk.Stack {
  public readonly assetBucket: s3.IBucket;
  public readonly domainName: string;
  public readonly distribution: cloudfront.Distribution;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    const { frontendVersion, ...restProps } = props;

    super(scope, id, restProps);

    this.assetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ACLS_ONLY,
      autoDeleteObjects: false, // Prevent accidental deletion of assets
    });

    // Create CloudFront origin without hardcoded version path
    // The Lambda function will update this dynamically
    const origin = origins.S3BucketOrigin.withOriginAccessControl(
      this.assetBucket,
      {
        originPath: `/${frontendVersion}`, // Initial path - will be updated by Lambda
      },
    );

    this.distribution = new cloudfront.Distribution(
      this,
      'FrontendDistribution',
      {
        comment:
          'Frontend Distribution for Glowing-Telegram with Dynamic Lambda Updates',
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

    this.domainName = this.distribution.distributionDomainName;

    // Create explicit log group for origin updater function
    const originUpdaterLogGroup = new logs.LogGroup(
      this,
      'OriginUpdaterLogGroup',
      {
        logGroupName: `${LOG_GROUP_PREFIX}/lambda/frontend-origin-updater`,
        retention: LOG_RETENTION,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      },
    );

    // Create Lambda function to handle CloudFront origin updates
    const originUpdaterFunction = new lambda.Function(
      this,
      'OriginUpdaterFunction',
      {
        runtime: lambda.Runtime.PYTHON_3_13,
        handler: 'index.handler',
        timeout: cdk.Duration.minutes(5),
        logGroup: originUpdaterLogGroup,
        tracing: lambda.Tracing.ACTIVE,
        loggingFormat: lambda.LoggingFormat.JSON,
        code: lambda.Code.fromInline(`
import json
import time
import os
import boto3
import logging
from urllib.parse import unquote_plus

# Configure logging
logger = logging.getLogger()
logger.setLevel(logging.INFO)

# AWS clients
cloudfront = boto3.client('cloudfront')
s3 = boto3.client('s3')

# Configuration constants
DISTRIBUTION_ID = os.environ.get('DISTRIBUTION_ID')
FALLBACK_VERSION = os.environ.get('FALLBACK_VERSION')

def handler(event, context):
    """
    Lambda function to update CloudFront distribution origin path
    when config/version.json is updated in S3
    """
    logger.info(f'Received event: {json.dumps(event)}')
    
    try:
        # Process S3 event records
        for record in event.get('Records', []):
            bucket_name = record['s3']['bucket']['name']
            object_key = unquote_plus(record['s3']['object']['key'])
            
            logger.info(f'Processing S3 event for bucket: {bucket_name}, key: {object_key}')
            
            # Check if this is a version config update
            if object_key == 'config/version.json':
                logger.info('Version config file updated, updating CloudFront distribution')
                update_cloudfront_origin(bucket_name, object_key)
            else:
                logger.info(f'Ignoring non-config file: {object_key}')
    
    except Exception as e:
        logger.error(f'Error processing event: {str(e)}')
        raise
    
    return {'statusCode': 200, 'body': 'Successfully processed S3 event'}

def update_cloudfront_origin(bucket_name, config_key):
    """
    Update CloudFront distribution origin path based on version config
    """
    try:
        # Read version config from S3
        version = get_version_from_s3(bucket_name, config_key)
        
        if not version:
            logger.warning(f'No version found in config, using fallback: {FALLBACK_VERSION}')
            version = FALLBACK_VERSION
        
        logger.info(f'Updating CloudFront distribution {DISTRIBUTION_ID} to version: {version}')
        
        # Get current distribution config
        response = cloudfront.get_distribution_config(Id=DISTRIBUTION_ID)
        config = response['DistributionConfig']
        etag = response['ETag']
        
        # Update the origin path for the first origin (S3 bucket origin)
        if config['Origins']['Items']:
            origin = config['Origins']['Items'][0]
            old_origin_path = origin.get('OriginPath', '')
            new_origin_path = f'/{version}'
            
            if old_origin_path != new_origin_path:
                origin['OriginPath'] = new_origin_path
                logger.info(f'Updating origin path from "{old_origin_path}" to "{new_origin_path}"')
                
                # Update the distribution
                update_response = cloudfront.update_distribution(
                    Id=DISTRIBUTION_ID,
                    DistributionConfig=config,
                    IfMatch=etag
                )
                
                logger.info(f'CloudFront distribution updated successfully. New ETag: {update_response["ETag"]}')
                
                # Create cache invalidation to ensure immediate updates
                invalidation_response = cloudfront.create_invalidation(
                    DistributionId=DISTRIBUTION_ID,
                    InvalidationBatch={
                        'Paths': {
                            'Quantity': 1,
                            'Items': ['/*']
                        },
                        'CallerReference': f'version-update-{int(time.time())}'
                    }
                )
                
                logger.info(f'Cache invalidation created: {invalidation_response["Invalidation"]["Id"]}')
            else:
                logger.info(f'Origin path already correct: {new_origin_path}')
        else:
            logger.error('No origins found in distribution config')
            
    except Exception as e:
        logger.error(f'Error updating CloudFront distribution: {str(e)}')
        raise

def get_version_from_s3(bucket_name, config_key):
    """
    Get version from S3 config file
    """
    try:
        response = s3.get_object(Bucket=bucket_name, Key=config_key)
        config_content = response['Body'].read().decode('utf-8')
        config_data = json.loads(config_content)
        
        version = config_data.get('version')
        if version:
            logger.info(f'Retrieved version from S3: {version}')
            return version
        else:
            logger.warning('No version field found in config file')
            return None
            
    except Exception as e:
        logger.error(f'Error reading version from S3: {str(e)}')
        return None
`),
        environment: {
          DISTRIBUTION_ID: this.distribution.distributionId,
          FALLBACK_VERSION: frontendVersion,
        },
      },
    );

    // Grant permissions to the Lambda function
    originUpdaterFunction.addToRolePolicy(
      new iam.PolicyStatement({
        effect: iam.Effect.ALLOW,
        actions: [
          'cloudfront:GetDistribution',
          'cloudfront:GetDistributionConfig',
          'cloudfront:UpdateDistribution',
          'cloudfront:CreateInvalidation',
        ],
        resources: [
          `arn:aws:cloudfront::${cdk.Stack.of(this).account}:distribution/${this.distribution.distributionId}`,
        ],
      }),
    );

    originUpdaterFunction.addToRolePolicy(
      new iam.PolicyStatement({
        effect: iam.Effect.ALLOW,
        actions: ['s3:GetObject'],
        resources: [`${this.assetBucket.bucketArn}/*`],
      }),
    );

    // Add S3 event notification to trigger Lambda when config/version.json changes
    this.assetBucket.addEventNotification(
      s3.EventType.OBJECT_CREATED,
      new s3n.LambdaDestination(originUpdaterFunction),
      {
        prefix: 'config/version.json',
      },
    );

    this.assetBucket.addEventNotification(
      s3.EventType.OBJECT_REMOVED,
      new s3n.LambdaDestination(originUpdaterFunction),
      {
        prefix: 'config/version.json',
      },
    );

    // Upload the default version config file
    new s3deployment.BucketDeployment(this, 'VersionConfigDeployment', {
      sources: [s3deployment.Source.asset(path.join(__dirname, '../config'))],
      destinationBucket: this.assetBucket,
      destinationKeyPrefix: 'config/',
    });
  }
}
