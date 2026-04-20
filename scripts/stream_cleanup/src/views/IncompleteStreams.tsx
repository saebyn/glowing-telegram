import { Box, Text, useInput } from 'ink';
import React, { useState } from 'react';
import { Spinner } from '../components/Spinner.js';
import { StreamTable } from '../components/StreamTable.js';
import type { ScanProgress } from '../services/types.js';
import type { ReconcileResult } from '../utils/reconcile.js';

interface IncompleteStreamsProps {
  reconcileResult: ReconcileResult;
  streamsScan: ScanProgress;
  onSelectStream: (streamId: string) => void;
  onBack: () => void;
}

export function IncompleteStreams({
  reconcileResult,
  streamsScan,
  onSelectStream,
  onBack,
}: IncompleteStreamsProps) {
  const { incompleteStreams } = reconcileResult;
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
      setSelectedIndex((i) => Math.min(incompleteStreams.length - 1, i + 1));
      return;
    }
    if (key.return && incompleteStreams.length > 0) {
      const item = incompleteStreams[selectedIndex];
      if (item) onSelectStream(item.stream.id);
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
          <Spinner label={`${incompleteStreams.length}+ found so far`} />
        ) : (
          <Text color="green">{incompleteStreams.length} total</Text>
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

      {incompleteStreams.length === 0 && !scanning ? (
        <Text color="green">✓ No incomplete streams found.</Text>
      ) : incompleteStreams.length === 0 ? (
        <Text dimColor>No incomplete streams found yet…</Text>
      ) : (
        <StreamTable
          streams={incompleteStreams}
          selectedIndex={selectedIndex}
        />
      )}

      <Box marginTop={1}>
        <Text dimColor>[↑/↓] navigate [Enter] edit stream [Esc] back</Text>
      </Box>
    </Box>
  );
}
