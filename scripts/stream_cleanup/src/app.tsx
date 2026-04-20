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
import { StreamDetail } from './views/StreamDetail.js';

type View =
  | 'dashboard'
  | 'incomplete-streams'
  | 'orphaned-files'
  | 'orphaned-streams'
  | 'stream-detail'
  | 'dry-run-summary';

interface AppProps {
  config: Config;
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

  // ---------- Create stream from orphaned S3 date ----------

  const handleCreateStream = useCallback(
    (date: string, files: S3VideoObject[]) => {
      addPendingChange({
        type: 'create_stream',
        description: `Create stream record for ${date} (${files.length} file(s))`,
        date,
        files,
      });
      goBack();
    },
    [addPendingChange, goBack],
  );

  // ---------- Apply all pending changes ----------

  const handleApply = useCallback(async () => {
    setApplyStatus('applying');
    try {
      for (const change of pendingChanges) {
        if (config.dryRun) {
          // In dry-run mode just log; no real writes.
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
            stream_date: change.date,
            prefix: `${change.date}/`,
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
              'orphaned-streams': 'orphaned-files', // reuse same view
              'count-mismatches': 'orphaned-files', // TODO: dedicated view
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
          onCreateStream={handleCreateStream}
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
