import * as cdk from 'aws-cdk-lib';
import { Template } from 'aws-cdk-lib/assertions';
import FrontendStack from '../lib/frontendStack';

describe('FrontendStack', () => {
  let app: cdk.App;
  let stack: FrontendStack;
  let template: Template;

  beforeEach(() => {
    app = new cdk.App();
    stack = new FrontendStack(app, 'TestFrontendStack', {
      frontendVersion: '1.2.3',
      env: { region: 'us-east-1' },
    });
    template = Template.fromStack(stack);
  });

  test('creates Lambda@Edge function with inlined code', () => {
    // Check that the Lambda function is created
    template.hasResourceProperties('AWS::Lambda::Function', {
      Runtime: 'python3.11',
      Handler: 'index.handler',
      Timeout: 5,
      MemorySize: 128,
    });
  });

  test('Lambda function code includes interpolated bucket name and fallback version', () => {
    // Get the Lambda function resource
    const lambdaFunctions = template.findResources('AWS::Lambda::Function');
    const lambdaFunction = Object.values(lambdaFunctions)[0];

    // The code is now a CloudFormation Fn::Join function that includes the bucket reference
    const code = lambdaFunction.Properties.Code.ZipFile;
    expect(code).toMatchObject({
      'Fn::Join': expect.arrayContaining([
        '',
        expect.arrayContaining([
          expect.stringContaining("BUCKET_NAME = '"),
          expect.objectContaining({
            Ref: expect.stringMatching(/FrontendAssetBucket/),
          }),
          expect.stringContaining("FALLBACK_VERSION = '1.2.3'"),
          expect.stringContaining("CONFIG_KEY = 'config/version.json'"),
          expect.stringContaining('def handler(event, context):'),
        ]),
      ]),
    });
  });

  test('creates CloudFront distribution with Lambda@Edge', () => {
    // Check that CloudFront distribution is created with Lambda@Edge
    template.hasResourceProperties('AWS::CloudFront::Distribution', {
      DistributionConfig: {
        DefaultCacheBehavior: {
          LambdaFunctionAssociations: [
            {
              EventType: 'viewer-request',
            },
          ],
        },
      },
    });
  });

  test('creates S3 bucket', () => {
    // Check that S3 bucket is created
    template.hasResourceProperties('AWS::S3::Bucket', {});
  });

  test('Lambda function has Lambda@Edge role', () => {
    // Check that there's a role with both lambda and edgelambda service principals
    const roles = template.findResources('AWS::IAM::Role');
    const versionSelectorRole = Object.values(roles).find((role) =>
      role.Properties?.AssumeRolePolicyDocument?.Statement?.some(
        (stmt: any) => stmt.Principal?.Service === 'edgelambda.amazonaws.com',
      ),
    );

    expect(versionSelectorRole).toBeDefined();
    expect(
      versionSelectorRole?.Properties?.AssumeRolePolicyDocument?.Statement,
    ).toHaveLength(2);
  });

  test('no environment variables are set on Lambda function', () => {
    // Verify that no environment variables are set (since Lambda@Edge doesn't support them)
    const lambdaFunctions = template.findResources('AWS::Lambda::Function');
    const lambdaFunction = Object.values(lambdaFunctions)[0];

    expect(lambdaFunction.Properties?.Environment).toBeUndefined();
  });
});
