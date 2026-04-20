import { useEffect, useState } from 'react';
import { listS3ObjectPages } from '../services/s3.js';
import type { S3VideoObject, ScanProgress } from '../services/types.js';
import type { Config } from '../utils/config.js';

export interface UseS3ObjectsResult {
  /** All S3 video objects grouped by YYYY-MM-DD date prefix. */
  s3ObjectsByDate: Map<string, S3VideoObject[]>;
  progress: ScanProgress;
}

/**
 * Progressively lists S3 objects one page at a time, building up the
 * date → files map incrementally. The map reference changes after each
 * page, triggering downstream memos (e.g. useReconcile) to recompute.
 */
export function useS3Objects(config: Config): UseS3ObjectsResult {
  const [s3ObjectsByDate, setS3ObjectsByDate] = useState<
    Map<string, S3VideoObject[]>
  >(new Map());
  const [progress, setProgress] = useState<ScanProgress>({
    status: 'idle',
    pagesScanned: 0,
    itemsScanned: 0,
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: config is stable CLI args; scan runs once on mount
  useEffect(() => {
    let cancelled = false;
    let totalItems = 0;

    async function scan() {
      setS3ObjectsByDate(new Map());
      setProgress({ status: 'scanning', pagesScanned: 0, itemsScanned: 0 });

      try {
        let page = 0;
        for await (const items of listS3ObjectPages(config)) {
          if (cancelled) return;

          page++;
          totalItems += items.length;

          setS3ObjectsByDate((prev) => {
            const next = new Map(prev);
            for (const obj of items) {
              const existing = next.get(obj.date) ?? [];
              next.set(obj.date, [...existing, obj]);
            }
            return next;
          });

          setProgress({
            status: 'scanning',
            pagesScanned: page,
            itemsScanned: totalItems,
          });
        }

        if (!cancelled) {
          setProgress((prev) => ({
            ...prev,
            status: 'complete',
          }));
        }
      } catch (err) {
        if (!cancelled) {
          setProgress((prev) => ({
            ...prev,
            status: 'error',
            error: String(err),
          }));
        }
      }
    }

    void scan();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { s3ObjectsByDate, progress };
}
