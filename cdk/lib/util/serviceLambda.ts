import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as ecr from 'aws-cdk-lib/aws-ecr';
import * as logs from 'aws-cdk-lib/aws-logs';

interface ServiceLambdaConstructProps {
  lambdaOptions: Omit<lambda.FunctionProps, 'code' | 'runtime' | 'handler'>;
  name: string;
  tagOrDigest?: string;
  imageVersion?: string;
}

export default class ServiceLambdaConstruct extends Construct {
  public readonly lambda: lambda.Function;
  public readonly repository: ecr.IRepository;

  constructor(
    scope: Construct,
    id: string,
    props: ServiceLambdaConstructProps,
  ) {
    super(scope, id);

    this.repository = ecr.Repository.fromRepositoryName(
      this,
      'Repository',
      `glowing-telegram/${props.name}`,
    );

    this.lambda = new lambda.Function(this, 'Lambda', {
      ...props.lambdaOptions,
      handler: lambda.Handler.FROM_IMAGE,
      runtime: lambda.Runtime.FROM_IMAGE,
      code: lambda.Code.fromEcrImage(this.repository, {
        tagOrDigest: props.tagOrDigest || props.imageVersion || 'latest',
      }),

      tracing: lambda.Tracing.ACTIVE,
      loggingFormat: lambda.LoggingFormat.JSON,
      logRetention: logs.RetentionDays.ONE_WEEK,
    });
  }
}
