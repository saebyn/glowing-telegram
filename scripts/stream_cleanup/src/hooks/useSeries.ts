import { useEffect, useState } from 'react';
import { scanSeriesPages } from '../services/dynamodb.js';
import type { ScanProgress, Series } from '../services/types.js';
import type { Config } from '../utils/config.js';

export interface UseSeriesResult {
  series: Series[];
  progress: ScanProgress;
}

/**
 * Progressively loads series records from DynamoDB. If no seriesTable is
 * configured the hook returns an empty list with status 'complete'.
 */
export function useSeries(config: Config): UseSeriesResult {
  const [series, setSeries] = useState<Series[]>([]);
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
      if (!config.seriesTable) {
        setProgress({ status: 'complete', pagesScanned: 0, itemsScanned: 0 });
        return;
      }

      setSeries([]);
      setProgress({ status: 'scanning', pagesScanned: 0, itemsScanned: 0 });

      try {
        let page = 0;
        for await (const items of scanSeriesPages(config)) {
          if (cancelled) return;

          page++;
          totalItems += items.length;
          setSeries((prev) => [...prev, ...items]);
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

  return { series, progress };
}
