import { type AttributeValue, DynamoDBClient } from '@aws-sdk/client-dynamodb';
import {
  DynamoDBDocumentClient,
  PutCommand,
  ScanCommand,
  type ScanCommandInput,
  UpdateCommand,
} from '@aws-sdk/lib-dynamodb';
import type { Config } from '../utils/config.js';
import type { Series, Stream } from './types.js';

function makeClient(config: Config): DynamoDBDocumentClient {
  const ddb = new DynamoDBClient({
    region: config.region,
  });
  return DynamoDBDocumentClient.from(ddb, {
    marshallOptions: { removeUndefinedValues: true },
  });
}

/**
 * Async generator that yields one page of Stream items at a time from
 * DynamoDB, allowing the caller to update UI state after each page.
 */
export async function* scanStreamPages(
  config: Config,
): AsyncGenerator<Stream[]> {
  const client = makeClient(config);
  let lastKey: Record<string, AttributeValue> | undefined;

  do {
    const input: ScanCommandInput = {
      TableName: config.streamsTable,
      ExclusiveStartKey: lastKey,
    };
    const result = await client.send(new ScanCommand(input));
    lastKey = result.LastEvaluatedKey as
      | Record<string, AttributeValue>
      | undefined;
    yield (result.Items ?? []) as Stream[];
  } while (lastKey);
}

/**
 * Async generator that yields one page of Series items at a time.
 */
export async function* scanSeriesPages(
  config: Config,
): AsyncGenerator<Series[]> {
  if (!config.seriesTable) return;

  const client = makeClient(config);
  let lastKey: Record<string, AttributeValue> | undefined;

  do {
    const input: ScanCommandInput = {
      TableName: config.seriesTable,
      ExclusiveStartKey: lastKey,
    };
    const result = await client.send(new ScanCommand(input));
    lastKey = result.LastEvaluatedKey as
      | Record<string, AttributeValue>
      | undefined;
    yield (result.Items ?? []) as Series[];
  } while (lastKey);
}

/** Update specific fields on an existing stream record. */
export async function updateStream(
  config: Config,
  streamId: string,
  updates: Partial<Stream>,
): Promise<void> {
  const client = makeClient(config);

  const entries = Object.entries(updates).filter(([, v]) => v !== undefined);
  if (entries.length === 0) return;

  const expressionParts: string[] = [];
  const names: Record<string, string> = {};
  const values: Record<string, unknown> = {};

  for (const [key, value] of entries) {
    const nameToken = `#${key}`;
    const valueToken = `:${key}`;
    expressionParts.push(`${nameToken} = ${valueToken}`);
    names[nameToken] = key;
    values[valueToken] = value;
  }

  values[':now'] = new Date().toISOString();
  names['#updated_at'] = 'updated_at';
  expressionParts.push('#updated_at = :now');

  await client.send(
    new UpdateCommand({
      TableName: config.streamsTable,
      Key: { id: streamId },
      UpdateExpression: `SET ${expressionParts.join(', ')}`,
      ExpressionAttributeNames: names,
      ExpressionAttributeValues: values,
    }),
  );
}

/** Create a brand-new stream record for a date that had no record. */
export async function createStream(
  config: Config,
  stream: Stream,
): Promise<void> {
  const client = makeClient(config);
  const now = new Date().toISOString();
  await client.send(
    new PutCommand({
      TableName: config.streamsTable,
      Item: {
        ...stream,
        created_at: stream.created_at ?? now,
        updated_at: now,
      },
      ConditionExpression: 'attribute_not_exists(id)',
    }),
  );
}
