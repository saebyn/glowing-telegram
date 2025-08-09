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
    // Lambda@Edge functions cannot use environment variables
    // Inline the Python code with interpolated bucket name and fallback version
    const pythonCode = `
import json
import time
import boto3
from botocore.exceptions import ClientError

# In-memory cache for version config
version_cache = {
    'version': None,
    'timestamp': 0,
    'ttl': 60000  # 60 seconds in milliseconds
}

# Configuration constants (interpolated at deploy time)
BUCKET_NAME = '${this.assetBucket.bucketName}'
FALLBACK_VERSION = '${frontendVersion}'
CONFIG_KEY = 'config/version.json'

def handler(event, context):
    """
    Lambda@Edge function to dynamically select frontend version
    This function intercepts CloudFront requests and rewrites the origin path
    based on the version specified in S3 config/version.json
    """
    request = event['Records'][0]['cf']['request']
    
    try:
        # Get current version from cache or S3
        current_version = get_current_version()
        
        # Use fallback version if no version found from S3
        version_to_use = current_version or FALLBACK_VERSION
        
        if not version_to_use:
            print('No version available, proceeding with original request')
            return request
        
        # Rewrite the origin path to include the version
        original_uri = request['uri']
        
        # Handle root path requests
        if original_uri == '/' or original_uri == '':
            request['uri'] = f'/{version_to_use}/index.html'
        else:
            # Prepend version to the path
            request['uri'] = f'/{version_to_use}{original_uri}'
        
        print(f'Rewritten URI from {original_uri} to {request["uri"]} for version {version_to_use}')
        
    except Exception as error:
        print(f'Error in version selector: {error}')
        # Use fallback version on error to maintain availability
        fallback_uri = request['uri']
        
        if FALLBACK_VERSION:
            if fallback_uri == '/' or fallback_uri == '':
                request['uri'] = f'/{FALLBACK_VERSION}/index.html'
            else:
                request['uri'] = f'/{FALLBACK_VERSION}{fallback_uri}'
            print(f'Using fallback version {FALLBACK_VERSION} due to error')
    
    return request

def get_current_version():
    """
    Get current version with caching
    """
    if not BUCKET_NAME:
        print('No bucket name available, cannot fetch version from S3')
        return None
        
    now = int(time.time() * 1000)  # Current time in milliseconds
    
    # Return cached version if still valid
    if version_cache['version'] and (now - version_cache['timestamp']) < version_cache['ttl']:
        print(f'Using cached version: {version_cache["version"]}')
        return version_cache['version']
    
    try:
        # Fetch new version from S3
        s3 = boto3.client('s3', region_name='us-east-1')  # Lambda@Edge requires us-east-1
        
        response = s3.get_object(Bucket=BUCKET_NAME, Key=CONFIG_KEY)
        config_data = json.loads(response['Body'].read().decode('utf-8'))
        
        if config_data.get('version'):
            # Update cache
            version_cache['version'] = config_data['version']
            version_cache['timestamp'] = now
            
            print(f'Fetched and cached new version: {config_data["version"]}')
            return config_data['version']
        else:
            print('Version not found in config file')
            return None
            
    except ClientError as error:
        print(f'Error fetching version from S3: {error}')
        
        # Return cached version if available, even if expired
        if version_cache['version']:
            print(f'Using stale cached version due to S3 error: {version_cache["version"]}')
            return version_cache['version']
        
        return None
    except Exception as error:
        print(f'Unexpected error fetching version from S3: {error}')
        
        # Return cached version if available, even if expired
        if version_cache['version']:
            print(f'Using stale cached version due to unexpected error: {version_cache["version"]}')
            return version_cache['version']
        
        return None

def reset_cache():
    """
    Reset cache for testing purposes
    """
    version_cache['version'] = None
    version_cache['timestamp'] = 0
`;

    this.versionSelectorFunction = new lambda.Function(this, 'VersionSelectorFunction', {
      runtime: lambda.Runtime.PYTHON_3_13,
      handler: 'index.handler',
      code: lambda.Code.fromInline(pythonCode),
      timeout: cdk.Duration.seconds(5),
      memorySize: 128,
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
