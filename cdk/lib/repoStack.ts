import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as iam from 'aws-cdk-lib/aws-iam';
import RepoConstruct from './util/repoConstruct';

interface RepoStackProps extends cdk.StackProps {
  frontendAssetBucket: s3.IBucket;
}

export default class RepoStack extends cdk.Stack {
  public readonly frontendAssetBucket: s3.IBucket;
  public readonly githubRole: iam.IRole;

  constructor(scope: Construct, id: string, props: RepoStackProps) {
    const { frontendAssetBucket, ...restProps } = props;
    super(scope, id, restProps);

    new RepoConstruct(this, 'RepoConstruct', {
      namespace: 'glowing-telegram',
      names: [
        'crud-lambda',
        'ai-chat-lambda',
        'summarize-transcription-lambda',
        'audio-transcription',
        'video-ingestor',
        'twitch-lambda',
        'youtube-lambda',
        'media-lambda',
        'render-job',
      ],
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
      StringLike: {
        'token.actions.githubusercontent.com:sub': `repo:${githubOrg}/${githubRepo}:environment:*`,
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
              resources: [frontendAssetBucket.arnForObjects('*')],
            }),
            new iam.PolicyStatement({
              actions: ['s3:ListBucket'],
              resources: [frontendAssetBucket.bucketArn],
            }),
          ],
        }),
      },
    });
  }
}
