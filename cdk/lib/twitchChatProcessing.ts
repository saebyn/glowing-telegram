import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as sqs from 'aws-cdk-lib/aws-sqs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as lambdaEventSources from 'aws-cdk-lib/aws-lambda-event-sources';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as iam from 'aws-cdk-lib/aws-iam';
import ServiceLambdaConstruct from './util/serviceLambda';

interface TwitchChatProcessingProps {
  chatMessagesTable: dynamodb.ITable;
  imageVersion?: string;
}

export default class TwitchChatProcessingConstruct extends Construct {
  public readonly chatQueue: sqs.IQueue;
  public readonly processingLambda: lambda.IFunction;

  constructor(scope: Construct, id: string, props: TwitchChatProcessingProps) {
    super(scope, id);

    // Create SQS queue for chat message processing
    this.chatQueue = new sqs.Queue(this, 'ChatMessagesQueue', {
      visibilityTimeout: cdk.Duration.minutes(5), // Should be 6x the lambda timeout
      deadLetterQueue: {
        queue: new sqs.Queue(this, 'ChatMessagesDeadLetterQueue', {
          retentionPeriod: cdk.Duration.days(14),
        }),
        maxReceiveCount: 3,
      },
      retentionPeriod: cdk.Duration.days(14),
    });

    // Create Lambda to process chat messages from SQS
    const chatProcessor = new ServiceLambdaConstruct(this, 'ChatProcessorLambda', {
      lambdaOptions: {
        description: 'Process Twitch chat messages from SQS and store in DynamoDB',
        timeout: cdk.Duration.seconds(30),
        environment: {
          CHAT_MESSAGES_TABLE: props.chatMessagesTable.tableName,
        },
      },
      name: 'chat-processor-lambda',
      imageVersion: props.imageVersion,
    });

    this.processingLambda = chatProcessor.lambda;

    // Add SQS event source to the Lambda
    this.processingLambda.addEventSource(
      new lambdaEventSources.SqsEventSource(this.chatQueue, {
        batchSize: 10, // Process up to 10 messages at once
        maxBatchingWindow: cdk.Duration.seconds(5),
      })
    );

    // Grant Lambda permissions to read from SQS
    this.chatQueue.grantConsumeMessages(this.processingLambda);

    // Grant Lambda permissions to write to DynamoDB
    this.processingLambda.addToRolePolicy(
      new iam.PolicyStatement({
        actions: [
          'dynamodb:PutItem',
          'dynamodb:UpdateItem',
          'dynamodb:GetItem',
        ],
        resources: [
          props.chatMessagesTable.tableArn,
        ],
      })
    );
  }
}