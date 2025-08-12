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
});
