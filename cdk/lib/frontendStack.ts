import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deployment from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as cr from 'aws-cdk-lib/custom-resources';
import * as path from 'node:path';

interface FrontendStackProps extends cdk.StackProps {
  // Keep frontendVersion for backwards compatibility, but it won't be used for origin path
  frontendVersion: string;
}

export default class FrontendStack extends cdk.Stack {
  public readonly assetBucket: s3.IBucket;
  public readonly domainName: string;
  public readonly versionSelectorFunction: lambda.IFunction;
  public readonly versionSelectorFunctionVersion: lambda.IVersion;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    const { frontendVersion, ...restProps } = props;

    // Lambda@Edge functions are automatically deployed to us-east-1 by CDK when used with CloudFront
    // No need to force the entire stack region - CDK handles Lambda@Edge replication automatically
    super(scope, id, restProps);

    this.assetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ACLS_ONLY,
      autoDeleteObjects: false, // Prevent accidental deletion of assets
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
        
        if object_exists(BUCKET_NAME, request['uri']):
          print(f'Rewritten URI from {original_uri} to {request["uri"]} for version {version_to_use}')
        else:
          # If the object does not exist, use /index.html as fallback
          request['uri'] = f'/{version_to_use}/index.html'
          print(f'Using fallback URI {request["uri"]} for version {version_to_use}')

    except Exception as error:
        print(f'Error in version selector: {error}')
        # Use fallback version on error to maintain availability
        fallback_uri = request['uri']
        
        if FALLBACK_VERSION:
            request['uri'] = f'/{FALLBACK_VERSION}/index.html'
            print(f'Using fallback version {FALLBACK_VERSION} due to error')
    
    return request

def object_exists(bucket, key):
    """    Check if an object exists in S3
    """
    s3 = boto3.client('s3', region_name='us-east-1')
    try:
        s3.head_object(Bucket=bucket, Key=key)
        return True
    except:
        return False

def get_current_version():
    """
    Get current version with caching
    """
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

    // Create IAM role for Lambda@Edge function
    const versionSelectorRole = new iam.Role(this, 'VersionSelectorRole', {
      assumedBy: new iam.CompositePrincipal(
        new iam.ServicePrincipal('lambda.amazonaws.com'),
        new iam.ServicePrincipal('edgelambda.amazonaws.com'),
      ),
      managedPolicies: [
        iam.ManagedPolicy.fromAwsManagedPolicyName(
          'service-role/AWSLambdaBasicExecutionRole',
        ),
      ],
      inlinePolicies: {
        S3ReadAccess: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({
              effect: iam.Effect.ALLOW,
              actions: ['s3:GetObject'],
              resources: [`${this.assetBucket.bucketArn}/*`],
            }),
          ],
        }),
      },
    });

    // Create custom resource to deploy Lambda@Edge function in us-east-1
    const lambdaEdgeProvider = new cr.Provider(this, 'LambdaEdgeProvider', {
      onEventHandler: new lambda.Function(this, 'LambdaEdgeHandler', {
        runtime: lambda.Runtime.PYTHON_3_11,
        handler: 'index.handler',
        timeout: cdk.Duration.minutes(5),
        code: lambda.Code.fromInline(`
import boto3
import json
import zipfile
import io
import base64
from urllib.request import Request, urlopen

def handler(event, context):
    """Custom resource handler for Lambda@Edge deployment in us-east-1"""
    print(f'Event: {json.dumps(event)}')
    
    request_type = event['RequestType']
    props = event['ResourceProperties']
    
    # Always use us-east-1 for Lambda@Edge
    lambda_client = boto3.client('lambda', region_name='us-east-1')
    
    try:
        if request_type == 'Create':
            return create_or_update_lambda_function(lambda_client, props, event)
        elif request_type == 'Update':
            return create_or_update_lambda_function(lambda_client, props, event)
        elif request_type == 'Delete':
            return delete_lambda_function(lambda_client, props, event)
    except Exception as e:
        print(f'Error: {str(e)}')
        send_response(event, context, 'FAILED', {'Error': str(e)})
        raise

def find_existing_function(lambda_client, function_name_prefix):
    """Find existing Lambda function matching the prefix pattern"""
    try:
        paginator = lambda_client.get_paginator('list_functions')
        for page in paginator.paginate():
            for func in page['Functions']:
                if func['FunctionName'].startswith(function_name_prefix):
                    print(f'Found existing function: {func["FunctionName"]}')
                    return func['FunctionName']
        return None
    except Exception as e:
        print(f'Error listing functions: {str(e)}')
        return None

def create_or_update_lambda_function(lambda_client, props, event):
    """Create or update Lambda@Edge function in us-east-1"""
    function_name_prefix = props['FunctionNamePrefix']
    python_code = props['Code']
    role_arn = props['RoleArn']
    
    # Look for existing function
    existing_function = find_existing_function(lambda_client, function_name_prefix)
    
    # Create zip file in memory
    zip_buffer = io.BytesIO()
    with zipfile.ZipFile(zip_buffer, 'w', zipfile.ZIP_DEFLATED) as zip_file:
        zip_file.writestr('index.py', python_code)
    
    zip_buffer.seek(0)
    zip_data = zip_buffer.read()
    
    if existing_function:
        # Update existing function
        print(f'Updating existing Lambda function: {existing_function}')
        lambda_client.update_function_code(
            FunctionName=existing_function,
            ZipFile=zip_data
        )
        
        # Publish new version
        version_response = lambda_client.publish_version(
            FunctionName=existing_function
        )
        
        function_arn = version_response['FunctionArn']
        version_arn = version_response['FunctionArn'] + ':' + version_response['Version']
        function_name = existing_function
        
        print(f'Updated Lambda function: {function_name}')
        print(f'New version ARN: {version_arn}')
    else:
        # Create new function with timestamp suffix for uniqueness
        import time
        timestamp_suffix = str(int(time.time()))
        function_name = f'{function_name_prefix}-{timestamp_suffix}'
        
        print(f'Creating new Lambda function: {function_name}')
        response = lambda_client.create_function(
            FunctionName=function_name,
            Runtime='python3.11',
            Role=role_arn,
            Handler='index.handler',
            Code={'ZipFile': zip_data},
            Timeout=5,
            MemorySize=128,
            Publish=True  # Publish version for Lambda@Edge
        )
        
        function_arn = response['FunctionArn']
        version_arn = response['FunctionArn'] + ':' + response['Version']
        
        print(f'Created Lambda function: {function_arn}')
        print(f'Version ARN: {version_arn}')
    
    return {
        'PhysicalResourceId': function_name,
        'Data': {
            'FunctionArn': function_arn,
            'VersionArn': version_arn,
            'FunctionName': function_name
        }
    }

def delete_lambda_function(lambda_client, props, event):
    """Delete Lambda@Edge function"""
    # Try to find the function by prefix since we might not have the exact name
    function_name_prefix = props.get('FunctionNamePrefix')
    physical_resource_id = event.get('PhysicalResourceId')
    
    # If we have the physical resource ID, try to delete that specific function
    if physical_resource_id and physical_resource_id != 'None':
        function_name = physical_resource_id
    else:
        # Fallback to finding by prefix
        function_name = find_existing_function(lambda_client, function_name_prefix)
    
    if function_name:
        try:
            lambda_client.delete_function(FunctionName=function_name)
            print(f'Successfully deleted Lambda function: {function_name}')
        except lambda_client.exceptions.ResourceNotFoundException:
            print(f'Function {function_name} not found, already deleted')
        except lambda_client.exceptions.InvalidParameterValueException as e:
            # Lambda@Edge functions cannot be deleted immediately due to replication
            print(f'Cannot delete Lambda@Edge function {function_name} yet (still replicated): {str(e)}')
            print('This is expected for Lambda@Edge functions and not an error')
        except lambda_client.exceptions.ResourceConflictException as e:
            # Function is still in use by CloudFront distributions
            print(f'Cannot delete Lambda@Edge function {function_name} (still in use): {str(e)}')
            print('This is expected for Lambda@Edge functions and not an error')
        except lambda_client.exceptions.TooManyRequestsException as e:
            # API throttling during deletion attempts
            print(f'API throttling during deletion of {function_name}: {str(e)}')
            print('This is expected during high API usage and not an error')
        except Exception as e:
            # For Lambda@Edge, catch any other AWS API errors that might occur during deletion
            # These are typically related to replication and edge location constraints
            print(f'Could not delete Lambda@Edge function {function_name}: {str(e)}')
            print('This is often expected for Lambda@Edge functions due to replication constraints')
            print('The function will eventually be cleaned up automatically by AWS')
    else:
        print('No function found to delete')
    
    # Always return success for delete operations of Lambda@Edge functions
    # CloudFormation stack deletion should not fail due to Lambda@Edge replication constraints
    return {'PhysicalResourceId': physical_resource_id or function_name_prefix}

def send_response(event, context, response_status, response_data):
    """Send response to CloudFormation"""
    response_url = event['ResponseURL']
    response_body = {
        'Status': response_status,
        'Reason': f'See CloudWatch Log Stream: {context.log_stream_name}',
        'PhysicalResourceId': event.get('PhysicalResourceId', context.log_stream_name),
        'StackId': event['StackId'],
        'RequestId': event['RequestId'],
        'LogicalResourceId': event['LogicalResourceId'],
        'Data': response_data
    }
    
    json_response_body = json.dumps(response_body)
    headers = {'content-type': '', 'content-length': str(len(json_response_body))}
    
    req = Request(response_url, data=json_response_body.encode('utf-8'), headers=headers)
    req.get_method = lambda: 'PUT'
    
    try:
        urlopen(req)
        print('Response sent successfully')
    except Exception as e:
        print(f'Error sending response: {e}')
        raise
`),
        initialPolicy: [
          new iam.PolicyStatement({
            effect: iam.Effect.ALLOW,
            actions: [
              'lambda:CreateFunction',
              'lambda:UpdateFunctionCode',
              'lambda:DeleteFunction',
              'lambda:PublishVersion',
              'lambda:GetFunction',
              'lambda:ListFunctions',
              'iam:PassRole',
            ],
            resources: [
              versionSelectorRole.roleArn,
              `arn:aws:lambda:us-east-1:${cdk.Stack.of(this).account}:function:${cdk.Stack.of(this).stackName}-VersionSelector*`,
              `arn:aws:lambda:us-east-1:${cdk.Stack.of(this).account}:function:*`,
            ],
          }),
        ],
      }),
    });

    // Custom resource to create Lambda@Edge function in us-east-1
    // Uses FunctionNamePrefix instead of FunctionName to enable finding and updating existing functions
    const versionSelectorCustomResource = new cdk.CustomResource(
      this,
      'VersionSelectorCustomResource',
      {
        serviceToken: lambdaEdgeProvider.serviceToken,
        properties: {
          FunctionNamePrefix: `${cdk.Stack.of(this).stackName}-VersionSelector`,
          Code: pythonCode,
          RoleArn: versionSelectorRole.roleArn,
        },
      },
    );

    // Create a version reference for Lambda@Edge
    this.versionSelectorFunctionVersion = lambda.Version.fromVersionArn(
      this,
      'VersionSelectorFunctionVersionRef',
      versionSelectorCustomResource.getAttString('VersionArn'),
    );

    // Add bucket policy to allow public read access to version config
    this.assetBucket.addToResourcePolicy(
      new iam.PolicyStatement({
        effect: iam.Effect.ALLOW,
        principals: [new iam.AnyPrincipal()],
        actions: ['s3:GetObject'],
        resources: [`${this.assetBucket.bucketArn}/*`],
      }),
    );

    // Create CloudFront origin without hardcoded version path
    const origin = origins.S3BucketOrigin.withOriginAccessControl(
      this.assetBucket,
      // No originPath - Lambda@Edge will handle version routing
    );

    const distribution = new cloudfront.Distribution(
      this,
      'FrontendDistribution',
      {
        comment:
          'Frontend Distribution for Glowing-Telegram with Dynamic Version Selection',
        defaultRootObject: 'index.html',
        defaultBehavior: {
          viewerProtocolPolicy:
            cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
          origin,
          // Add Lambda@Edge function for viewer request
          edgeLambdas: [
            {
              functionVersion: this.versionSelectorFunctionVersion,
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

/**
 * Generates a random 8-character alphanumeric string.
 * Used to create unique identifiers for Lambda function naming to avoid conflicts.
 */
function randomId() {
  // return a random 8-character alphanumeric string
  return Math.random().toString(36).substring(2, 10);
}
