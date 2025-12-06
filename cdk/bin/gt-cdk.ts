#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import AppStack from '../lib/appStack';
import RepoStack from '../lib/repoStack';
import FrontendStack from '../lib/frontendStack';
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

new RepoStack(app, getStackName('RepoStack', environmentName), {
  env: {
    account: environmentConfig.awsAccount,
    region: environmentConfig.awsRegion,
  },
  frontendAssetBucket: frontendStack.assetBucket,
  environmentName,
  tags: environmentConfig.tags,
});

new AppStack(app, getStackName('AppStack', environmentName), {
  env: {
    account: environmentConfig.awsAccount,
    region: environmentConfig.awsRegion,
  },
  domainName: frontendStack.domainName,
  tagOrDigest: process.env.IMAGE_VERSION,
  environmentName,
  tags: environmentConfig.tags,
});
