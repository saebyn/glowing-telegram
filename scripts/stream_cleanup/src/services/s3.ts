import { ListObjectsV2Command, S3Client } from '@aws-sdk/client-s3';
import type { Config } from '../utils/config.js';
import type { S3VideoObject } from './types.js';

/**
 * Matches keys of the form: YYYY-MM-DD/anything
 * The date portion is captured as group 1.
 */
const KEY_PATTERN = /^(\d{4}-\d{2}-\d{2})\//;

/**
 * Async generator that yields one page of parsed S3VideoObjects at a time.
 * Keys that don't match the expected date-prefix format are silently skipped
 * to avoid triggering Glacier retrieval costs from unexpected object types.
 */
export async function* listS3ObjectPages(
  config: Config,
): AsyncGenerator<S3VideoObject[]> {
  const client = new S3Client({ region: config.region });
  let continuationToken: string | undefined;

  do {
    const result = await client.send(
      new ListObjectsV2Command({
        Bucket: config.bucket,
        Prefix: config.prefix,
        ContinuationToken: continuationToken,
      }),
    );

    const objects: S3VideoObject[] = [];
    for (const obj of result.Contents ?? []) {
      if (!obj.Key) continue;
      const match = obj.Key.match(KEY_PATTERN);
      if (!match) continue;

      objects.push({
        key: obj.Key,
        date: match[1],
        filename: obj.Key.slice(match[0].length),
        size: obj.Size,
        lastModified: obj.LastModified,
      });
    }

    yield objects;
    continuationToken = result.NextContinuationToken;
  } while (continuationToken);
}
