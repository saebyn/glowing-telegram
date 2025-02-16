import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import type * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import type * as s3 from 'aws-cdk-lib/aws-s3';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as lambda from 'aws-cdk-lib/aws-lambda';

import ServiceLambdaConstruct from './util/serviceLambda';

interface MediaServeConstructProps {
  mediaOutputBucket: s3.IBucket;
  videoMetadataTable: dynamodb.ITable;
  domainName: string;
}

export default class MediaServeConstruct extends Construct {
  domainName: string;

  constructor(scope: Construct, id: string, props: MediaServeConstructProps) {
    super(scope, id);

    const { mediaOutputBucket, videoMetadataTable, domainName } = props;

    const mediaOrigin = origins.S3BucketOrigin.withOriginAccessControl(
      mediaOutputBucket,
      {
        originPath: '/transcode',
      },
    );

    const responseHeadersPolicy = new cloudfront.ResponseHeadersPolicy(
      this,
      'MediaResponseHeadersPolicy',
      {
        corsBehavior: {
          accessControlAllowOrigins: [
            'http://localhost:5173',
            `https://${domainName}`,
          ],
          accessControlAllowMethods: ['GET', 'OPTIONS'],
          accessControlAllowHeaders: ['*'],
          accessControlAllowCredentials: false,
          originOverride: true,
          accessControlExposeHeaders: [
            'Content-Length',
            'Content-Range',
            'Content-Type',
          ],
          accessControlMaxAge: cdk.Duration.days(10),
        },
      },
    );

    const distribution = new cloudfront.Distribution(
      this,
      'MediaDistribution',
      {
        comment: 'Media Distribution for Glowing-Telegram',
        defaultBehavior: {
          origin: mediaOrigin,
          // trustedKeyGroups
          viewerProtocolPolicy:
            cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
          responseHeadersPolicy,
        },
      },
    );

    const playlistLambda = new ServiceLambdaConstruct(
      this,
      'PlaylistMediaLambda',
      {
        lambdaOptions: {
          description: 'Media Lambda for Glowing-Telegram',
          timeout: cdk.Duration.seconds(10),
          environment: {
            VIDEO_METADATA_TABLE: videoMetadataTable.tableName,
            STREAM_ID_INDEX: 'stream_id-index',
          },
        },
        name: 'media-lambda',
      },
    );

    videoMetadataTable.grantReadData(playlistLambda.lambda);

    const playlistLambdaUrl = playlistLambda.lambda.addFunctionUrl({
      authType: lambda.FunctionUrlAuthType.AWS_IAM,
      cors: {
        allowedOrigins: ['http://localhost:5173', `https://${domainName}`],
      },
    });

    const playlistOrigin =
      origins.FunctionUrlOrigin.withOriginAccessControl(playlistLambdaUrl);

    distribution.addBehavior('/playlist/*.m3u8', playlistOrigin);

    this.domainName = distribution.distributionDomainName;
  }
}
