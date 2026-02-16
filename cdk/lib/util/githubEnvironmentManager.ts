import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as cr from 'aws-cdk-lib/custom-resources';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import { LOG_GROUP_PREFIX, LOG_RETENTION } from './serviceLambda';

export interface GitHubEnvironmentManagerProps {
  /**
   * The GitHub repository owner/org
   */
  owner: string;

  /**
   * The GitHub repository name
   */
  repo: string;

  /**
   * The environment name (dev, staging, production)
   */
  environmentName: string;

  /**
   * Variables to set on the GitHub environment
   */
  variables: Record<string, string>;

  /**
   * GitHub Personal Access Token secret
   * The token needs repo scope
   */
  githubTokenSecret: secretsmanager.ISecret;
}

/**
 * Custom resource to manage GitHub environments and their variables
 */
export class GitHubEnvironmentManager extends Construct {
  constructor(scope: Construct, id: string, props: GitHubEnvironmentManagerProps) {
    super(scope, id);

    // Use the provided GitHub token secret
    const githubTokenSecret = props.githubTokenSecret;

    // Create log group for the custom resource Lambda
    const logGroup = new logs.LogGroup(this, 'LogGroup', {
      logGroupName: `${LOG_GROUP_PREFIX}/custom-resources/github-env-manager-${props.repo}-${props.environmentName}`,
      retention: LOG_RETENTION,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Lambda function to manage GitHub environments
    const handlerFunction = new lambda.Function(this, 'Handler', {
      runtime: lambda.Runtime.PYTHON_3_13,
      handler: 'index.handler',
      timeout: cdk.Duration.minutes(5),
      logGroup,
      environment: {
        GITHUB_TOKEN_SECRET_ARN: githubTokenSecret.secretArn,
      },
      code: lambda.Code.fromInline(`
import json
import os
import boto3
import urllib3
import logging
from urllib.parse import quote

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize clients
secrets_client = boto3.client('secretsmanager')
http = urllib3.PoolManager()

def get_github_token():
    """Retrieve GitHub token from Secrets Manager"""
    secret_arn = os.environ['GITHUB_TOKEN_SECRET_ARN']
    try:
        response = secrets_client.get_secret_value(SecretId=secret_arn)
        secret = json.loads(response['SecretString']) if 'SecretString' in response else {}
        return secret.get('token') or response.get('SecretString', '')
    except Exception as e:
        logger.error(f"Failed to retrieve GitHub token: {str(e)}")
        raise

def github_api_request(method, url, token, data=None):
    """Make a request to GitHub API"""
    headers = {
        'Authorization': f'Bearer {token}',
        'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
        'User-Agent': 'AWS-CDK-CustomResource/1.0'
    }
    
    body = json.dumps(data).encode('utf-8') if data else None
    
    try:
        response = http.request(method, url, headers=headers, body=body)
        logger.info(f"{method} {url} - Status: {response.status}")
        
        if response.status >= 200 and response.status < 300:
            return json.loads(response.data.decode('utf-8')) if response.data else {}
        else:
            error_msg = response.data.decode('utf-8') if response.data else 'No error message'
            logger.error(f"GitHub API error: {error_msg}")
            raise Exception(f"GitHub API request failed with status {response.status}: {error_msg}")
    except Exception as e:
        logger.error(f"Request failed: {str(e)}")
        raise

def create_or_update_environment(owner, repo, env_name, token):
    """Create or update a GitHub environment"""
    url = f"https://api.github.com/repos/{owner}/{repo}/environments/{quote(env_name)}"
    
    # Try to get existing environment first
    try:
        existing = github_api_request('GET', url, token)
        logger.info(f"Environment {env_name} already exists")
        return existing
    except Exception:
        logger.info(f"Environment {env_name} does not exist, creating...")
    
    # Create or update the environment
    data = {
        'wait_timer': 0,
        'deployment_branch_policy': None  # Allow all branches
    }
    
    return github_api_request('PUT', url, token, data)

def set_environment_variable(owner, repo, env_name, var_name, var_value, token):
    """Set a variable on a GitHub environment"""
    url = f"https://api.github.com/repos/{owner}/{repo}/environments/{quote(env_name)}/variables/{var_name}"
    
    data = {
        'name': var_name,
        'value': var_value
    }
    
    # Try to update first
    try:
        return github_api_request('PATCH', url, token, data)
    except Exception:
        # If update fails, try to create
        url = f"https://api.github.com/repos/{owner}/{repo}/environments/{quote(env_name)}/variables"
        return github_api_request('POST', url, token, data)

def delete_environment(owner, repo, env_name, token):
    """Delete a GitHub environment"""
    url = f"https://api.github.com/repos/{owner}/{repo}/environments/{quote(env_name)}"
    try:
        github_api_request('DELETE', url, token)
        logger.info(f"Deleted environment {env_name}")
    except Exception as e:
        logger.warning(f"Failed to delete environment {env_name}: {str(e)}")

def handler(event, context):
    """Lambda handler for custom resource"""
    logger.info(f"Received event: {json.dumps(event)}")
    
    request_type = event['RequestType']
    properties = event['ResourceProperties']
    
    owner = properties['Owner']
    repo = properties['Repo']
    env_name = properties['EnvironmentName']
    variables = json.loads(properties['Variables'])
    
    response_data = {}
    
    try:
        token = get_github_token()
        
        if request_type in ['Create', 'Update']:
            # Create or update the environment
            create_or_update_environment(owner, repo, env_name, token)
            
            # Set each variable
            for var_name, var_value in variables.items():
                logger.info(f"Setting variable {var_name} on {env_name}")
                set_environment_variable(owner, repo, env_name, var_name, var_value, token)
            
            response_data['Message'] = f"Successfully configured environment {env_name}"
            
        elif request_type == 'Delete':
            # Optionally delete the environment on stack deletion
            # Commented out to preserve environments by default
            # delete_environment(owner, repo, env_name, token)
            response_data['Message'] = f"Environment {env_name} preserved (not deleted)"
        
        return {
            'Status': 'SUCCESS',
            'PhysicalResourceId': f"{owner}/{repo}/environments/{env_name}",
            'Data': response_data
        }
        
    except Exception as e:
        logger.error(f"Error: {str(e)}")
        return {
            'Status': 'FAILED',
            'Reason': str(e),
            'PhysicalResourceId': f"{owner}/{repo}/environments/{env_name}",
            'Data': {}
        }
`),
    });

    // Grant permissions to read the secret
    githubTokenSecret.grantRead(handlerFunction);

    // Create custom resource provider
    const provider = new cr.Provider(this, 'Provider', {
      onEventHandler: handlerFunction,
      logGroup,
    });

    // Create the custom resource
    new cdk.CustomResource(this, 'Resource', {
      serviceToken: provider.serviceToken,
      properties: {
        Owner: props.owner,
        Repo: props.repo,
        EnvironmentName: props.environmentName,
        Variables: JSON.stringify(props.variables),
        // Add a timestamp to force updates when variables change
        UpdateTimestamp: Date.now().toString(),
      },
    });
  }
}
