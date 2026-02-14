#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import AppStack from '../lib/appStack';
import RepoStack from '../lib/repoStack';
import FrontendStack from '../lib/frontendStack';
import GitHubEnvironmentStack from '../lib/githubEnvironmentStack';
import { loadEnvironmentConfig, getStackName } from '../lib/util/environment';

const app = new cdk.App();

// Load environment configuration
const { name: environmentName, config: environmentConfig } = loadEnvironmentConfig();

console.log(`Deploying to environment: ${environmentName}`);
console.log(`Configuration:`, JSON.stringify(environmentConfig, null, 2));

// Override frontendVersion from environment variable if provided
const frontendVersion = process.env.FRONTEND_VERSION || environmentConfig.frontendVersion;

const frontendStack = new FrontendStack(app, getStackName('FrontendStack', environmentName), {
  env: {
    account: environmentConfig.awsAccount,
    region: environmentConfig.awsRegion,
  },
  frontendVersion,
  environmentName,
  tags: environmentConfig.tags,
});

const repoStack = new RepoStack(app, getStackName('RepoStack', environmentName), {
  env: {
    account: environmentConfig.awsAccount,
    region: environmentConfig.awsRegion,
  },
  frontendAssetBucket: frontendStack.assetBucket,
  environmentName,
  tags: environmentConfig.tags,
});

const appStack = new AppStack(app, getStackName('AppStack', environmentName), {
  env: {
    account: environmentConfig.awsAccount,
    region: environmentConfig.awsRegion,
  },
  domainName: frontendStack.domainName,
  tagOrDigest: process.env.IMAGE_VERSION,
  environmentName,
  tags: environmentConfig.tags,
});

// Create GitHub environments stack - depends on AppStack and FrontendStack
// This will be deployed after the other stacks to capture their outputs
// Set SKIP_GITHUB_ENV=true to skip GitHub environment creation
if (process.env.SKIP_GITHUB_ENV !== 'true') {
  const githubEnvStack = new GitHubEnvironmentStack(app, getStackName('GitHubEnvironmentStack', environmentName), {
    env: {
      account: environmentConfig.awsAccount,
      region: environmentConfig.awsRegion,
    },
    environmentName,
    // Pass references from other stacks - CDK will handle the cross-stack references
    apiUrl: appStack.apiUrl,
    websocketUrl: appStack.websocketUrl,
    userPoolId: appStack.userPoolId,
    userPoolClientId: appStack.userPoolClientId,
    cognitoDomain: appStack.cognitoDomain,
    awsRegion: environmentConfig.awsRegion,
    contentUrl: appStack.contentUrl,
    redirectUri: appStack.redirectUri,
    logoutUri: appStack.logoutUri,
    frontendBucketName: frontendStack.assetBucket.bucketName,
    githubRoleArn: repoStack.githubRole.roleArn,
    siteDomain: frontendStack.domainName,
    twitchClientId: process.env.TWITCH_CLIENT_ID,
    tags: environmentConfig.tags,
  });
  
  // Ensure GitHub environment stack deploys after the main stacks
  githubEnvStack.addDependency(appStack);
  githubEnvStack.addDependency(frontendStack);
  githubEnvStack.addDependency(repoStack);
}
