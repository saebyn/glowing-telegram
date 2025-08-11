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
  public readonly dockerGithubRole: iam.IRole;

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
        'upload-video',
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
        // CDK deployment requires CloudFormation permissions
        iam.ManagedPolicy.fromAwsManagedPolicyName(
          'CloudWatchFullAccess',
        ),
        // For comprehensive CDK deployment capabilities
        iam.ManagedPolicy.fromAwsManagedPolicyName(
          'PowerUserAccess',
        ),
      ],
      inlinePolicies: {
        CDKDeploymentPolicy: new iam.PolicyDocument({
          statements: [
            // CloudFormation permissions for CDK
            new iam.PolicyStatement({
              actions: [
                'cloudformation:CreateStack',
                'cloudformation:UpdateStack',
                'cloudformation:DeleteStack',
                'cloudformation:DescribeStacks',
                'cloudformation:DescribeStackEvents',
                'cloudformation:DescribeStackResources',
                'cloudformation:GetTemplate',
                'cloudformation:ListStacks',
                'cloudformation:ListStackResources',
                'cloudformation:CreateChangeSet',
                'cloudformation:DescribeChangeSet',
                'cloudformation:ExecuteChangeSet',
                'cloudformation:DeleteChangeSet',
                'cloudformation:GetStackPolicy',
                'cloudformation:SetStackPolicy',
                'cloudformation:ValidateTemplate',
              ],
              resources: ['*'],
            }),
            // IAM permissions for CDK to manage roles and policies
            new iam.PolicyStatement({
              actions: [
                'iam:CreateRole',
                'iam:UpdateRole',
                'iam:DeleteRole',
                'iam:GetRole',
                'iam:PassRole',
                'iam:CreateInstanceProfile',
                'iam:DeleteInstanceProfile',
                'iam:AddRoleToInstanceProfile',
                'iam:RemoveRoleFromInstanceProfile',
                'iam:AttachRolePolicy',
                'iam:DetachRolePolicy',
                'iam:PutRolePolicy',
                'iam:DeleteRolePolicy',
                'iam:GetRolePolicy',
                'iam:ListRolePolicies',
                'iam:ListAttachedRolePolicies',
                'iam:CreatePolicy',
                'iam:DeletePolicy',
                'iam:GetPolicy',
                'iam:CreatePolicyVersion',
                'iam:DeletePolicyVersion',
                'iam:ListPolicyVersions',
                'iam:SetDefaultPolicyVersion',
                'iam:CreateOpenIDConnectProvider',
                'iam:DeleteOpenIDConnectProvider',
                'iam:GetOpenIDConnectProvider',
                'iam:UpdateOpenIDConnectProviderThumbprint',
                'iam:TagRole',
                'iam:UntagRole',
                'iam:TagPolicy',
                'iam:UntagPolicy',
                'iam:TagOpenIDConnectProvider',
                'iam:UntagOpenIDConnectProvider',
              ],
              resources: ['*'],
            }),
            // SSM permissions for CDK bootstrap
            new iam.PolicyStatement({
              actions: [
                'ssm:GetParameter',
                'ssm:PutParameter',
                'ssm:DeleteParameter',
              ],
              resources: [
                `arn:aws:ssm:*:${cdk.Aws.ACCOUNT_ID}:parameter/cdk-bootstrap/*`,
              ],
            }),
          ],
        }),
      },
    });
  }
}
