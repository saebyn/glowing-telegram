import { Construct } from 'constructs';

import * as cognito from 'aws-cdk-lib/aws-cognito';

export default class UserpoolConstruct extends Construct {
  public readonly userPool: cognito.IUserPool;
  public readonly userPoolClient: cognito.IUserPoolClient;

  constructor(scope: Construct, id: string) {
    super(scope, id);

    this.userPool = cognito.UserPool.fromUserPoolId(
      this,
      'UserPool',
      'us-west-2_MTXvnFJfB',
    );

    this.userPoolClient = cognito.UserPoolClient.fromUserPoolClientId(
      this,
      'UserPoolClient',
      '476l1b3p98vffdnfrll0a5llup',
    );
  }
}
