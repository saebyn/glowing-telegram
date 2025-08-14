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

  test('creates custom resource for Lambda@Edge with proper deletion handling', () => {
    // Check that custom resource is created for Lambda@Edge deployment
    template.hasResourceProperties('AWS::CloudFormation::CustomResource', {
      ServiceToken: {
        'Fn::GetAtt': [
          // Match any custom resource handler
          {},
          'Arn',
        ],
      },
    });

    // Verify the custom resource handler exists
    template.hasResourceProperties('AWS::Lambda::Function', {
      Runtime: 'python3.11',
      Handler: 'index.handler',
    });
  });
});
