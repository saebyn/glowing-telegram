import type { S3VideoObject, Stream } from '../services/types.js';

// Fields that must be present on a Stream for it to be considered complete.
export const REQUIRED_FIELDS: (keyof Stream)[] = [
  'stream_date',
  'title',
  'prefix'
];

export interface ReconcileResult {
  incompleteStreams: Array<{ stream: Stream; missingFields: string[] }>;
  /**
   * S3 date-prefixes with no matching stream record.
   * Only populated once the DynamoDB streams scan is complete; a partially-
   * scanned streams table could produce false positives here.
   */
  orphanedS3Dates: Array<{ date: string; files: S3VideoObject[] }>;
  /**
   * Streams whose prefix has no S3 files.
   * Only populated once the S3 scan is complete.
   */
  orphanedStreams: Array<Stream>;
}

/** Extract the YYYY-MM-DD date key used to cross-reference against S3. */
export function getStreamDateKey(stream: Stream): string | undefined {
  // Prefer the prefix as the canonical key — use it exactly as stored.
  if (stream.prefix) return stream.prefix;

  // Fall back to stream_date, converting to US/Pacific to get the local date.
  if (stream.stream_date) {
    return new Date(stream.stream_date).toLocaleDateString('en-CA', {
      timeZone: 'America/Los_Angeles',
    });
  }

  return undefined;
}

export function reconcile(
  streams: Stream[],
  s3ObjectsByDate: Map<string, S3VideoObject[]>,
  requiredFields: (keyof Stream)[],
  scanState: { streamsComplete: boolean; s3Complete: boolean },
): ReconcileResult {
  const { streamsComplete, s3Complete } = scanState;

  // Build a lookup from date key → stream for cross-referencing.
  const streamsByDate = new Map<string, Stream>();
  for (const stream of streams) {
    const date = getStreamDateKey(stream);
    if (date) streamsByDate.set(date, stream);
  }

  // --- Incomplete streams ---
  // Reliable for every stream we've seen so far, regardless of scan state.
  // New incomplete streams may still be discovered while DynamoDB is scanning.
  const incompleteStreams: ReconcileResult['incompleteStreams'] = [];
  for (const stream of streams) {
    const missingFields = requiredFields.filter((field) => {
      const val = stream[field];
      return val === undefined || val === null || val === '';
    });
    if (missingFields.length > 0) {
      incompleteStreams.push({
        stream,
        missingFields: missingFields as string[],
      });
    }
  }

  // --- Orphaned S3 dates ---
  // An S3 date-prefix with no matching stream record is only reliable once
  // the DynamoDB scan is complete. Before that, the "missing" stream might
  // simply not have been scanned yet.
  const orphanedS3Dates: ReconcileResult['orphanedS3Dates'] = streamsComplete
    ? [...s3ObjectsByDate.entries()]
        .filter(([date]) => !streamsByDate.has(date))
        .map(([date, files]) => ({ date, files }))
    : [];

  // --- Orphaned streams ---
  // A stream with no S3 files is only reliable once the S3 scan is complete.
  const orphanedStreams: ReconcileResult['orphanedStreams'] = s3Complete
    ? streams.filter((stream) => {
        const date = getStreamDateKey(stream);
        if (!date) return false;
        return !s3ObjectsByDate.has(date);
      })
    : [];

  return {
    incompleteStreams,
    orphanedS3Dates,
    orphanedStreams,
  };
}
