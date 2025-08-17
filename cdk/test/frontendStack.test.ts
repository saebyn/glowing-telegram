import * as cdk from 'aws-cdk-lib';
import { Template, Match } from 'aws-cdk-lib/assertions';
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

  test('creates CloudFront distribution with static origin path', () => {
    // Check that CloudFront distribution is created with origin path
    template.hasResourceProperties('AWS::CloudFront::Distribution', {
      DistributionConfig: {
        DefaultCacheBehavior: {
          // Should not have Lambda@Edge function associations
          ViewerProtocolPolicy: 'redirect-to-https',
        },
        // Check that origin has a path
        Origins: [
          {
            S3OriginConfig: Match.anyValue(),
            OriginPath: '/0.4.0', // Should match version from config file
          },
        ],
      },
    });
  });

  test('creates S3 bucket', () => {
    // Check that S3 bucket is created
    template.hasResourceProperties('AWS::S3::Bucket', {});
  });

  test('does not create Lambda@Edge function or custom resources', () => {
    // Verify no Lambda@Edge custom resource is created
    template.resourcePropertiesCountIs('AWS::CloudFormation::CustomResource', {}, 0);
    
    // Verify no Lambda function is created specifically for Lambda@Edge version selection
    // Note: BucketDeployment creates its own Lambda function which is expected
    const lambdaFunctions = template.findResources('AWS::Lambda::Function');
    const versionSelectorFunctions = Object.values(lambdaFunctions).filter((resource: any) => 
      resource.Properties?.Code?.ZipFile && 
      typeof resource.Properties.Code.ZipFile === 'string' &&
      resource.Properties.Code.ZipFile.includes('version-selector')
    );
    expect(versionSelectorFunctions).toHaveLength(0);
  });
});
