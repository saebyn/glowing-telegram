import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as iam from 'aws-cdk-lib/aws-iam';
import RepoConstruct from './util/repoConstruct';

export default class RepoStack extends cdk.Stack {
  public readonly frontendAssetBucket: s3.IBucket;
  githubRole: iam.IRole;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    new RepoConstruct(this, 'RepoConstruct', {
      namespace: 'glowing-telegram',
      names: [
        'crud-lambda',
        'ai-chat-lambda',
        'summarize-transcription-lambda',
        'audio-transcription',
        'video-ingestor',
        'twitch-lambda',
      ],
    });

    this.frontendAssetBucket = new s3.Bucket(this, 'FrontendAssetBucket', {
      versioned: false,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    const audience = 'sts.amazonaws.com';
    const githubOrg = 'saebyn';
    const githubRepo = 'glowing-telegram-frontend';

    const provider = new iam.OpenIdConnectProvider(this, 'GithubOIDCProvider', {
      url: 'https://token.actions.githubusercontent.com',
      clientIds: ['sts.amazonaws.com'],
      thumbprints: ['ffffffffffffffffffffffffffffffffffffffff'],
    });

    const principal = new iam.OpenIdConnectPrincipal(provider, {
      StringEquals: {
        'token.actions.githubusercontent.com:sub': `repo:${githubOrg}/${githubRepo}`,
        'token.actions.githubusercontent.com:aud': audience,
      },
    });

    this.githubRole = new iam.Role(this, 'GithubActionRole', {
      assumedBy: principal,
      inlinePolicies: {
        GithubActionPolicy: new iam.PolicyDocument({
          statements: [
            new iam.PolicyStatement({
              actions: ['s3:PutObject'],
              resources: [this.frontendAssetBucket.arnForObjects('*')],
            }),
          ],
        }),
      },
    });
  }
}
