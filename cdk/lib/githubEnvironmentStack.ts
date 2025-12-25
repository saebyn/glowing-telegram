import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import { GitHubEnvironmentManager } from './util/githubEnvironmentManager';

interface GitHubEnvironmentStackProps extends cdk.StackProps {
  environmentName: string;
  apiUrl: string;
  websocketUrl: string;
  userPoolId: string;
  userPoolClientId: string;
  cognitoDomain: string;
  awsRegion: string;
  contentUrl: string;
  redirectUri: string;
  logoutUri: string;
  frontendBucketName: string;
  githubRoleArn: string;
  siteDomain: string;
  // Optional Twitch client ID - can be set via secret or parameter
  twitchClientId?: string;
  // GitHub organization/owner name
  githubOwner?: string;
}

export default class GitHubEnvironmentStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: GitHubEnvironmentStackProps) {
    const {
      environmentName,
      apiUrl,
      websocketUrl,
      userPoolId,
      userPoolClientId,
      cognitoDomain,
      awsRegion,
      contentUrl,
      redirectUri,
      logoutUri,
      frontendBucketName,
      githubRoleArn,
      siteDomain,
      twitchClientId,
      githubOwner,
      ...restProps
    } = props;

    super(scope, id, restProps);

    // Get GitHub owner from props or environment variable, default to 'saebyn'
    const owner = githubOwner || process.env.GITHUB_OWNER || 'saebyn';

    // Get or create GitHub token secret
    // The secret must contain a GitHub Personal Access Token with repo scope
    const githubTokenSecret = secretsmanager.Secret.fromSecretNameV2(
      this,
      'GitHubTokenSecret',
      'glowing-telegram/github-token'
    );

    // Use provided Twitch client ID or placeholder
    const twitchClientIdValue = twitchClientId || 'TWITCH_CLIENT_ID_NOT_SET';

    // Prepare variables for frontend repository
    const frontendVariables: Record<string, string> = {
      API_URL: apiUrl,
      AWS_REGION: awsRegion,
      AWS_ROLE_ARN: githubRoleArn,
      BUCKET_NAME: frontendBucketName,
      COGNITO_CLIENT_ID: userPoolClientId,
      COGNITO_DOMAIN: cognitoDomain,
      COGNITO_USER_POOL_ID: userPoolId,
      CONTENT_URL: contentUrl,
      LOGOUT_URI: logoutUri,
      REDIRECT_URI: redirectUri,
      SITE_DOMAIN: siteDomain,
      TWITCH_CLIENT_ID: twitchClientIdValue,
      WEBSOCKET_URL: websocketUrl,
    };

    // Create/update GitHub environment for frontend repository
    new GitHubEnvironmentManager(this, 'FrontendEnvironment', {
      owner: owner,
      repo: 'glowing-telegram-frontend',
      environmentName: environmentName,
      variables: frontendVariables,
      githubTokenSecretArn: githubTokenSecret.secretArn,
    });

    // Prepare variables for backend repository (this repo)
    // For the backend, we mainly need to know it's a valid environment
    const backendVariables: Record<string, string> = {
      ENVIRONMENT: environmentName,
      AWS_REGION: awsRegion,
    };

    // Create/update GitHub environment for backend repository
    new GitHubEnvironmentManager(this, 'BackendEnvironment', {
      owner: owner,
      repo: 'glowing-telegram',
      environmentName: environmentName,
      variables: backendVariables,
      githubTokenSecretArn: githubTokenSecret.secretArn,
    });

    // Output the environment name for reference
    new cdk.CfnOutput(this, 'GitHubEnvironmentName', {
      value: environmentName,
      description: 'GitHub Environment name configured',
    });

    new cdk.CfnOutput(this, 'FrontendRepoEnvironment', {
      value: `https://github.com/${owner}/glowing-telegram-frontend/settings/environments`,
      description: 'Frontend repository environments URL',
    });

    new cdk.CfnOutput(this, 'BackendRepoEnvironment', {
      value: `https://github.com/${owner}/glowing-telegram/settings/environments`,
      description: 'Backend repository environments URL',
    });
  }
}
