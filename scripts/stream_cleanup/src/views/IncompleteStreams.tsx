import { Box, Text, useInput } from 'ink';
import React, { useMemo, useState } from 'react';
import { Spinner } from '../components/Spinner.js';
import { StreamTable } from '../components/StreamTable.js';
import type { ScanProgress } from '../services/types.js';
import { type ReconcileResult, getStreamDateKey } from '../utils/reconcile.js';

interface IncompleteStreamsProps {
  reconcileResult: ReconcileResult;
  streamsScan: ScanProgress;
  queuedStreamIds: Set<string>;
  onSelectStream: (streamId: string) => void;
  onBack: () => void;
}

export function IncompleteStreams({
  reconcileResult,
  streamsScan,
  queuedStreamIds,
  onSelectStream,
  onBack,
}: IncompleteStreamsProps) {
  const { incompleteStreams } = reconcileResult;
  const sortedIncompleteStreams = useMemo(
    () =>
      [...incompleteStreams].sort((left, right) => {
        const leftDate = getStreamDateKey(left.stream) ?? '9999-99-99';
        const rightDate = getStreamDateKey(right.stream) ?? '9999-99-99';
        if (leftDate !== rightDate) return leftDate.localeCompare(rightDate);
        return left.stream.id.localeCompare(right.stream.id);
      }),
    [incompleteStreams],
  );
  const [selectedIndex, setSelectedIndex] = useState(0);
  const scanning = streamsScan.status === 'scanning';

  useInput((_input, key) => {
    if (key.escape) {
      onBack();
      return;
    }
    if (key.upArrow) {
      setSelectedIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIndex((i) =>
        Math.min(sortedIncompleteStreams.length - 1, i + 1),
      );
      return;
    }
    if (key.return && sortedIncompleteStreams.length > 0) {
      const item = sortedIncompleteStreams[selectedIndex];
      if (item && !queuedStreamIds.has(item.stream.id)) {
        onSelectStream(item.stream.id);
      }
    }
  });

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Header */}
      <Box marginBottom={1} gap={2}>
        <Text bold color="cyan">
          Incomplete Streams
        </Text>
        {scanning ? (
          <Spinner label={`${sortedIncompleteStreams.length}+ found so far`} />
        ) : (
          <Text color="green">{sortedIncompleteStreams.length} total</Text>
        )}
      </Box>

      {/* Scan-in-progress banner */}
      {scanning && (
        <Box marginBottom={1}>
          <Text color="yellow">
            ⚠ DynamoDB scan still running — this list is growing.
            {'\n'} Fixes queued now are flagged for re-review once scanning
            {'\n'} completes. The list updates automatically.
          </Text>
        </Box>
      )}

      {sortedIncompleteStreams.length === 0 && !scanning ? (
        <Text color="green">✓ No incomplete streams found.</Text>
      ) : sortedIncompleteStreams.length === 0 ? (
        <Text dimColor>No incomplete streams found yet…</Text>
      ) : (
        <StreamTable
          streams={sortedIncompleteStreams}
          selectedIndex={selectedIndex}
          queuedStreamIds={queuedStreamIds}
        />
      )}

      <Box marginTop={1}>
        <Text dimColor>
          [↑/↓] navigate [Enter] edit unqueued stream [Esc] back
        </Text>
      </Box>
    </Box>
  );
}
