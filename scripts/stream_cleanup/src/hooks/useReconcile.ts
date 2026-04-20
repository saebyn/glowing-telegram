import { useMemo } from 'react';
import type { S3VideoObject, ScanProgress, Stream } from '../services/types.js';
import {
  REQUIRED_FIELDS,
  type ReconcileResult,
  reconcile,
} from '../utils/reconcile.js';

/**
 * Reactively recomputes the reconcile result whenever streams, S3 objects,
 * or scan-completion status changes. Because reconcile() is a pure function,
 * React.useMemo provides the correct dependency-driven update semantics.
 *
 * Cross-referencing results (orphaned files/streams, count mismatches) are
 * suppressed until the relevant scan is complete to avoid false positives.
 */
export function useReconcile(
  streams: Stream[],
  s3ObjectsByDate: Map<string, S3VideoObject[]>,
  streamsScan: ScanProgress,
  s3Scan: ScanProgress,
): ReconcileResult {
  const streamsComplete = streamsScan.status === 'complete';
  const s3Complete = s3Scan.status === 'complete';

  return useMemo(
    () =>
      reconcile(streams, s3ObjectsByDate, REQUIRED_FIELDS, {
        streamsComplete,
        s3Complete,
      }),
    // s3ObjectsByDate is a new Map reference after each page, so this works.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [streams, s3ObjectsByDate, streamsComplete, s3Complete],
  );
}
