import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';

import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';

export default class DatastoreConstruct extends Construct {
  public readonly videoArchive: s3.IBucket;
  public readonly outputBucket: s3.IBucket;
  public readonly episodesTable: dynamodb.ITable;
  public readonly profilesTable: dynamodb.ITable;
  public readonly streamSeriesTable: dynamodb.ITable;
  public readonly streamsTable: dynamodb.ITable;
  public readonly videoMetadataTable: dynamodb.ITable;

  constructor(scope: Construct, id: string) {
    super(scope, id);

    // TODO import the resources from the existing Pulumi stack

    this.videoArchive = s3.Bucket.fromBucketName(
      this,
      'VideoArchive',
      'saebyn-video-archive',
    );

    this.outputBucket = s3.Bucket.fromBucketName(
      this,
      'OutputBucket',
      'output-bucket-ded3bd2',
    );

    this.episodesTable = dynamodb.Table.fromTableName(
      this,
      'EpisodesTable',
      'episodes-03b1f6f',
    );

    this.profilesTable = dynamodb.Table.fromTableName(
      this,
      'ProfilesTable',
      'profiles-323335b',
    );

    this.streamSeriesTable = dynamodb.Table.fromTableName(
      this,
      'StreamSeriesTable',
      'stream-series-09d6bad',
    );

    this.streamsTable = dynamodb.Table.fromTableName(
      this,
      'StreamsTable',
      'streams-963700c',
    );

    this.videoMetadataTable = dynamodb.Table.fromTableName(
      this,
      'VideoMetadataTable',
      'metadata-table-aa16405',
    );
  }
}
