import { useEffect, useState } from 'react';
import { scanStreamPages } from '../services/dynamodb.js';
import type { ScanProgress, Stream } from '../services/types.js';
import type { Config } from '../utils/config.js';

export interface UseStreamsResult {
  streams: Stream[];
  progress: ScanProgress;
}

/**
 * Progressively scans the DynamoDB streams table one page at a time.
 * State is updated after each page, so the UI can show live counts and
 * let the user start reviewing incomplete streams before the scan finishes.
 */
export function useStreams(config: Config): UseStreamsResult {
  const [streams, setStreams] = useState<Stream[]>([]);
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
      setStreams([]);
      setProgress({ status: 'scanning', pagesScanned: 0, itemsScanned: 0 });

      try {
        let page = 0;
        for await (const items of scanStreamPages(config)) {
          if (cancelled) return;

          page++;
          totalItems += items.length;
          setStreams((prev) => [...prev, ...items]);
          setProgress({
            status: 'scanning',
            pagesScanned: page,
            itemsScanned: totalItems,
          });
        }

        if (!cancelled) {
          setProgress({
            status: 'complete',
            pagesScanned: page,
            itemsScanned: totalItems,
          });
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

  return { streams, progress };
}
