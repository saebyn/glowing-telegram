import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as cognito from 'aws-cdk-lib/aws-cognito';

export default class UserpoolConstruct extends Construct {
  public readonly userPool: cognito.IUserPool;
  public readonly userPoolDomain: cognito.IUserPoolDomain;
  public readonly userPoolClient: cognito.IUserPoolClient;

  constructor(scope: Construct, id: string) {
    super(scope, id);

    this.userPool = new cognito.UserPool(this, 'UserPool', {
      accountRecovery: cognito.AccountRecovery.EMAIL_ONLY,
      autoVerify: { email: true },
      deletionProtection: true,
      deviceTracking: {
        challengeRequiredOnNewDevice: true,
        deviceOnlyRememberedOnUserPrompt: true,
      },
      email: cognito.UserPoolEmail.withCognito(),
      mfa: cognito.Mfa.OPTIONAL,
      passwordPolicy: {
        minLength: 20,
        tempPasswordValidity: cdk.Duration.days(1),
        requireDigits: true,
        requireLowercase: true,
        requireSymbols: true,
        requireUppercase: true,
      },
      mfaSecondFactor: {
        sms: false,
        otp: true,
      },
      signInAliases: {
        email: true,
      },
      signInCaseSensitive: false,
      selfSignUpEnabled: false,
      featurePlan: cognito.FeaturePlan.ESSENTIALS,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    this.userPoolDomain = new cognito.UserPoolDomain(this, 'UserPoolDomain', {
      userPool: this.userPool,
      cognitoDomain: {
        domainPrefix: 'glowing-telegram',
      },
    });

    this.userPoolClient = new cognito.UserPoolClient(this, 'UserPoolClient', {
      userPool: this.userPool,
      authFlows: {
        adminUserPassword: true,
        custom: false,
        userPassword: true,
        userSrp: true,
        user: true,
      },
      generateSecret: false,
      oAuth: {
        callbackUrls: [
          'http://localhost:5173/auth-callback',
          'https://localhost:5173/auth-callback',
          'http://localhost:5173/',
          'https://localhost:5173/',
        ],
        logoutUrls: ['http://localhost:5173/', 'https://localhost:5173/'],
        flows: {
          authorizationCodeGrant: true,
          implicitCodeGrant: false,
          clientCredentials: false,
        },
        scopes: [
          cognito.OAuthScope.COGNITO_ADMIN,
          cognito.OAuthScope.EMAIL,
          cognito.OAuthScope.OPENID,
          cognito.OAuthScope.PHONE,
          cognito.OAuthScope.PROFILE,
        ],
      },
      preventUserExistenceErrors: true,
      accessTokenValidity: cdk.Duration.hours(1),
      idTokenValidity: cdk.Duration.hours(1),
      refreshTokenValidity: cdk.Duration.days(5),
      supportedIdentityProviders: [
        cognito.UserPoolClientIdentityProvider.COGNITO,
      ],
      enableTokenRevocation: true,
    });

    this.userPoolClient.applyRemovalPolicy(cdk.RemovalPolicy.RETAIN);
  }
}
