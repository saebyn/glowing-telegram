import { randomUUID } from 'node:crypto';
import { Box, Text, useApp } from 'ink';
import React, { useState, useCallback } from 'react';
import { useReconcile } from './hooks/useReconcile.js';
import { useS3Objects } from './hooks/useS3Objects.js';
import { useSeries } from './hooks/useSeries.js';
import { useStreams } from './hooks/useStreams.js';
import { createStream, updateStream } from './services/dynamodb.js';
import type {
  PendingChange,
  PendingChangeInput,
  S3VideoObject,
  Stream,
} from './services/types.js';
import type { Config } from './utils/config.js';
import { Dashboard } from './views/Dashboard.js';
import { DryRunSummary } from './views/DryRunSummary.js';
import { IncompleteStreams } from './views/IncompleteStreams.js';
import { OrphanedFiles } from './views/OrphanedFiles.js';
import { OrphanedStreams } from './views/OrphanedStreams.js';
import { StreamDetail } from './views/StreamDetail.js';

type View =
  | 'dashboard'
  | 'incomplete-streams'
  | 'orphaned-files'
  | 'orphaned-streams'
  | 'stream-detail'
  | 'count-mismatches'
  | 'dry-run-summary';

interface AppProps {
  config: Config;
}

const S3_LOCAL_TIMESTAMP_PATTERN =
  /^(\d{4})-(\d{2})-(\d{2}) (\d{2})-(\d{2})-(\d{2})\.[^/]+$/;

function parseShortOffsetToMinutes(offsetText: string): number | undefined {
  const match = offsetText.match(/^GMT([+-])(\d{1,2})(?::(\d{2}))?$/);
  if (!match) return undefined;

  const sign = match[1] === '+' ? 1 : -1;
  const hours = Number.parseInt(match[2], 10);
  const minutes = Number.parseInt(match[3] ?? '0', 10);
  return sign * (hours * 60 + minutes);
}

function getTimeZoneOffsetMinutes(
  date: Date,
  timeZone: string,
): number | undefined {
  const formatter = new Intl.DateTimeFormat('en-US', {
    timeZone,
    timeZoneName: 'shortOffset',
  });

  const offsetPart = formatter
    .formatToParts(date)
    .find((part) => part.type === 'timeZoneName')?.value;

  if (!offsetPart) return undefined;
  return parseShortOffsetToMinutes(offsetPart);
}

function pacificLocalToUtcIso(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  second: number,
): string | undefined {
  const timeZone = 'America/Los_Angeles';
  const naiveUtcMs = Date.UTC(year, month - 1, day, hour, minute, second);

  const firstOffset = getTimeZoneOffsetMinutes(new Date(naiveUtcMs), timeZone);
  if (firstOffset === undefined) return undefined;

  let utcMs = naiveUtcMs - firstOffset * 60_000;
  const secondOffset = getTimeZoneOffsetMinutes(new Date(utcMs), timeZone);
  if (secondOffset !== undefined && secondOffset !== firstOffset) {
    utcMs = naiveUtcMs - secondOffset * 60_000;
  }

  return new Date(utcMs).toISOString();
}

function deriveStreamDateFromFirstS3File(
  files: S3VideoObject[],
  fallbackDate: string,
): string {
  const firstKey = files[0]?.key;
  const firstFilename = files[0]?.filename;
  const match = firstFilename?.match(S3_LOCAL_TIMESTAMP_PATTERN);

  if (match) {
    const iso = pacificLocalToUtcIso(
      Number.parseInt(match[1], 10),
      Number.parseInt(match[2], 10),
      Number.parseInt(match[3], 10),
      Number.parseInt(match[4], 10),
      Number.parseInt(match[5], 10),
      Number.parseInt(match[6], 10),
    );
    if (iso) return iso;
  }

  // Fallback: midnight US/Pacific for the prefix date.
  const fallbackMatch = fallbackDate.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (fallbackMatch) {
    const iso = pacificLocalToUtcIso(
      Number.parseInt(fallbackMatch[1], 10),
      Number.parseInt(fallbackMatch[2], 10),
      Number.parseInt(fallbackMatch[3], 10),
      0,
      0,
      0,
    );
    if (iso) return iso;
  }

  console.warn(
    '[stream-cleanup] Failed to parse Pacific timestamp from first S3 key; falling back to date string:',
    firstKey ?? '(missing key)',
  );
  return fallbackDate;
}

export function App({ config }: AppProps) {
  const { exit } = useApp();

  // Progressive scans — state updates arrive page-by-page.
  const { streams, progress: streamsScan } = useStreams(config);
  const { s3ObjectsByDate, progress: s3Scan } = useS3Objects(config);
  const { series } = useSeries(config);

  // Reactively recomputed after every page. Cross-referencing results are
  // suppressed until the relevant scan completes to avoid false positives.
  const reconcileResult = useReconcile(
    streams,
    s3ObjectsByDate,
    streamsScan,
    s3Scan,
  );

  const bothScansComplete =
    streamsScan.status === 'complete' && s3Scan.status === 'complete';

  // Navigation & pending changes state.
  const [view, setView] = useState<View>('dashboard');
  const [selectedStreamId, setSelectedStreamId] = useState<string | null>(null);
  const [pendingChanges, setPendingChanges] = useState<PendingChange[]>([]);
  const [applyStatus, setApplyStatus] = useState<
    'idle' | 'applying' | 'done' | 'error'
  >('idle');
  const [applyError, setApplyError] = useState<string | null>(null);

  // ---------- Navigation helpers ----------

  const goTo = useCallback((v: View) => setView(v), []);
  const goBack = useCallback(() => setView('dashboard'), []);

  // ---------- Pending changes helpers ----------

  const addPendingChange = useCallback(
    (change: PendingChangeInput) => {
      setPendingChanges((prev) => [
        ...prev,
        {
          ...change,
          id: randomUUID(),
          createdWithScansComplete: bothScansComplete,
        } as PendingChange,
      ]);
    },
    [bothScansComplete],
  );

  const discardChange = useCallback((id: string) => {
    setPendingChanges((prev) => prev.filter((c) => c.id !== id));
  }, []);

  const queuedCreateDates = pendingChanges.reduce<Set<string>>(
    (acc, change) => {
      if (change.type === 'create_stream') acc.add(change.date);
      return acc;
    },
    new Set<string>(),
  );

  const queuedIncompleteStreamIds = pendingChanges.reduce<Set<string>>(
    (acc, change) => {
      if (change.type === 'update_stream') acc.add(change.streamId);
      return acc;
    },
    new Set<string>(),
  );

  // ---------- Create stream from orphaned S3 date ----------

  const handleCreateStream = useCallback(
    (date: string, files: S3VideoObject[]) => {
      addPendingChange({
        type: 'create_stream',
        description: `Create stream record for ${date} (${files.length} file(s))`,
        date,
        stream_date: deriveStreamDateFromFirstS3File(files, date),
        files,
      });
    },
    [addPendingChange],
  );

  // ---------- Apply all pending changes ----------

  const handleApply = useCallback(async () => {
    setApplyStatus('applying');
    try {
      for (const change of pendingChanges) {
        if (config.dryRun) {
          // In dry-run mode just log; no real writes.
          console.log('[Dry-run] Would apply change:', change);
          continue;
        }

        if (change.type === 'update_stream') {
          await updateStream(config, change.streamId, change.updates);
        } else if (change.type === 'update_count') {
          await updateStream(config, change.streamId, {
            video_clip_count: change.newCount,
          });
        } else if (change.type === 'create_stream') {
          const now = new Date().toISOString();
          const newStream: Stream = {
            id: randomUUID(),
            title: `Stream ${change.date}`,
            stream_date: change.stream_date,
            prefix: `${change.date}`,
            stream_platform: 'twitch',
            video_clip_count: change.files.length,
            created_at: now,
          };
          await createStream(config, newStream);
        }
      }
      setPendingChanges([]);
      setApplyStatus('done');
    } catch (err) {
      setApplyError(String(err));
      setApplyStatus('error');
    }
  }, [config, pendingChanges]);

  // ---------- After apply: show result then return to dashboard ----------

  if (applyStatus === 'done') {
    return (
      <Box flexDirection="column" paddingX={1}>
        <Text color="green">
          ✓ {config.dryRun ? 'Dry-run simulation' : 'Changes applied'}{' '}
          successfully.
        </Text>
        <Text dimColor>Press any key to return to the dashboard.</Text>
      </Box>
    );
  }

  if (applyStatus === 'error') {
    return (
      <Box flexDirection="column" paddingX={1}>
        <Text color="red">✗ Error applying changes: {applyError}</Text>
        <Text dimColor>Press any key to return to the dashboard.</Text>
      </Box>
    );
  }

  if (applyStatus === 'applying') {
    return (
      <Box paddingX={1}>
        <Text color="cyan">⠋ Applying {pendingChanges.length} change(s)…</Text>
      </Box>
    );
  }

  // ---------- Render current view ----------

  const selectedStream = selectedStreamId
    ? (streams.find((s) => s.id === selectedStreamId) ?? null)
    : null;

  return (
    <Box flexDirection="column">
      {view === 'dashboard' && (
        <Dashboard
          reconcileResult={reconcileResult}
          streamsScan={streamsScan}
          s3Scan={s3Scan}
          pendingChanges={pendingChanges}
          onSelectView={(v) => {
            if (v === 'exit') {
              exit();
              return;
            }
            // Map dashboard view names to internal view names.
            const mapping: Record<string, View> = {
              'incomplete-streams': 'incomplete-streams',
              'orphaned-files': 'orphaned-files',
              'orphaned-streams': 'orphaned-streams',
              'count-mismatches': 'count-mismatches',
              'dry-run-summary': 'dry-run-summary',
            };
            goTo(mapping[v] ?? 'dashboard');
          }}
        />
      )}

      {view === 'incomplete-streams' && (
        <IncompleteStreams
          reconcileResult={reconcileResult}
          streamsScan={streamsScan}
          queuedStreamIds={queuedIncompleteStreamIds}
          onSelectStream={(id) => {
            setSelectedStreamId(id);
            goTo('stream-detail');
          }}
          onBack={goBack}
        />
      )}

      {view === 'orphaned-files' && (
        <OrphanedFiles
          reconcileResult={reconcileResult}
          streamsScan={streamsScan}
          s3Scan={s3Scan}
          bothScansComplete={bothScansComplete}
          queuedCreateDates={queuedCreateDates}
          onCreateStream={handleCreateStream}
          onBack={goBack}
        />
      )}

      {view === 'orphaned-streams' && (
        <OrphanedStreams
          reconcileResult={reconcileResult}
          s3Scan={s3Scan}
          onBack={goBack}
        />
      )}

      {view === 'stream-detail' && selectedStream && (
        <StreamDetail
          stream={selectedStream}
          series={series}
          bothScansComplete={bothScansComplete}
          onQueueChange={(change) => {
            addPendingChange(change);
            goTo('incomplete-streams');
          }}
          onBack={() => goTo('incomplete-streams')}
        />
      )}

      {view === 'dry-run-summary' && (
        <DryRunSummary
          pendingChanges={pendingChanges}
          streams={streams}
          bothScansComplete={bothScansComplete}
          dryRun={config.dryRun}
          onApply={() => void handleApply()}
          onDiscard={discardChange}
          onBack={goBack}
        />
      )}
    </Box>
  );
}
