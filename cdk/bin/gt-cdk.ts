#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import AppStack from '../lib/appStack';
import RepoStack from '../lib/repoStack';

const app = new cdk.App();

const repoStack = new RepoStack(app, 'RepoStack', {});

new AppStack(app, 'AppStack', {
  frontendAssetBucket: repoStack.frontendAssetBucket,
  frontendVersion: process.env.FRONTEND_VERSION || '0.1.0',
});
