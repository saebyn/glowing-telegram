import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as iam from 'aws-cdk-lib/aws-iam';
import RepoConstruct from './util/repoConstruct';

interface RepoStackProps extends cdk.StackProps {
  frontendAssetBucket: s3.IBucket;
  environmentName: string;
}

export default class RepoStack extends cdk.Stack {
  public readonly frontendAssetBucket: s3.IBucket;
  public readonly githubRole: iam.IRole;
  public readonly dockerGithubRole: iam.IRole;

  constructor(scope: Construct, id: string, props: RepoStackProps) {
    const { frontendAssetBucket, environmentName, ...restProps } = props;
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
        'youtube-uploader-lambda',
        'media-lambda',
        'render-job',
        'upload-video',
        'chat-processor-lambda',
        'embedding-service',
        'websocket-lambda',
        'widget-updater-lambda',
        'ingestion-management-lambda',
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

    // Use environment-specific role name for non-production environments
    const githubRoleName = environmentName === 'production'
      ? undefined // Use CDK-generated name for production (existing role)
      : `GlowingTelegram-GithubActionRole-${environmentName}`;

    this.githubRole = new iam.Role(this, 'GithubActionRole', {
      roleName: githubRoleName,
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

    // GitHub Actions role for Docker image builds in the main repository
    const dockerPrincipal = new iam.OpenIdConnectPrincipal(provider, {
      StringLike: {
        'token.actions.githubusercontent.com:sub': `repo:${githubOrg}/glowing-telegram:*`,
        'token.actions.githubusercontent.com:aud': audience,
      },
    });

    this.dockerGithubRole = new iam.Role(this, 'DockerGithubActionRole', {
      roleName: 'GlowingTelegram-DockerGithubActionRole',
      description: 'Role for GitHub Actions to build Docker images and deploy CDK',
      assumedBy: dockerPrincipal,
      managedPolicies: [
        iam.ManagedPolicy.fromAwsManagedPolicyName(
          'AmazonEC2ContainerRegistryPowerUser',
        ),
      ],
      inlinePolicies: {
        CDKAssumeRolePolicy: new iam.PolicyDocument({
          statements: [
            // CDK v2 manages its own roles, we only need permission to assume them
            new iam.PolicyStatement({
              actions: [
                'sts:AssumeRole',
              ],
              resources: [
                'arn:aws:iam::*:role/cdk-*',
              ],
            }),
          ],
        }),
      },
    });

    // Export stack outputs for GitHub configuration
    // Only export for production to avoid CloudFormation export name conflicts
    // Non-production environments can use CloudFormation describe-stacks API
    if (environmentName === 'production') {
      new cdk.CfnOutput(this, 'GithubActionRoleArn', {
        value: this.githubRole.roleArn,
        description: 'IAM Role ARN for GitHub Actions (frontend deployment)',
        exportName: 'GlowingTelegram-GithubActionRoleArn',
      });

      new cdk.CfnOutput(this, 'DockerGithubActionRoleArn', {
        value: this.dockerGithubRole.roleArn,
        description: 'IAM Role ARN for GitHub Actions (Docker builds and CDK deployment)',
        exportName: 'GlowingTelegram-DockerGithubActionRoleArn',
      });
    } else {
      // Output without export name for non-production environments
      new cdk.CfnOutput(this, 'GithubActionRoleArn', {
        value: this.githubRole.roleArn,
        description: `IAM Role ARN for GitHub Actions (frontend deployment) - ${environmentName}`,
      });

      new cdk.CfnOutput(this, 'DockerGithubActionRoleArn', {
        value: this.dockerGithubRole.roleArn,
        description: `IAM Role ARN for GitHub Actions (Docker builds and CDK deployment) - ${environmentName}`,
      });
    }
  }
}
