#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import AppStack from '../lib/appStack';
import RepoStack from '../lib/repoStack';
import FrontendStack from '../lib/frontendStack';

const app = new cdk.App();

const frontendStack = new FrontendStack(app, 'FrontendStack', {
  frontendVersion: process.env.FRONTEND_VERSION || '0.4.1',
});

new RepoStack(app, 'RepoStack', {
  frontendAssetBucket: frontendStack.assetBucket,
});

new AppStack(app, 'AppStack', {
  domainName: frontendStack.domainName,
  imageVersion: process.env.IMAGE_VERSION,
});
