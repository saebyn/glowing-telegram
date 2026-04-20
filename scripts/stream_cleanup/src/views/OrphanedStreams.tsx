import { Box, Text, useInput } from 'ink';
import React, { useState } from 'react';
import { Spinner } from '../components/Spinner.js';
import type { ScanProgress } from '../services/types.js';
import type { ReconcileResult } from '../utils/reconcile.js';

interface OrphanedStreamsProps {
  reconcileResult: ReconcileResult;
  s3Scan: ScanProgress;
  onBack: () => void;
}

export function OrphanedStreams({
  reconcileResult,
  s3Scan,
  onBack,
}: OrphanedStreamsProps) {
  const { orphanedStreams } = reconcileResult;
  const [selectedIndex, setSelectedIndex] = useState(0);
  const s3Scanning = s3Scan.status === 'scanning';

  useInput((_input, key) => {
    if (key.escape) {
      onBack();
      return;
    }
    if (key.upArrow) {
      setSelectedIndex((i) => Math.max(0, i - 1));
    }
    if (key.downArrow) {
      setSelectedIndex((i) => Math.min(orphanedStreams.length - 1, i + 1));
    }
  });

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Header */}
      <Box marginBottom={1} gap={2}>
        <Text bold color="cyan">
          Orphaned Streams
        </Text>
        {s3Scanning && <Spinner />}
      </Box>

      {s3Scanning && (
        <Box marginBottom={1}>
          <Text color="yellow">
            ⚠ S3 scan still running — a stream that appears orphaned here may
            {'\n'} have files that haven't been scanned yet. This list will
            {'\n'} update automatically.
          </Text>
        </Box>
      )}

      {!s3Scanning && orphanedStreams.length === 0 ? (
        <Text color="green">✓ No orphaned stream records found.</Text>
      ) : orphanedStreams.length === 0 ? (
        <Text dimColor>
          Waiting for S3 scan to finish before showing results…
        </Text>
      ) : (
        <Box flexDirection="column">
          {orphanedStreams.map((stream, i) => (
            <Box key={stream.id} gap={2}>
              <Text color={i === selectedIndex ? 'cyan' : undefined}>
                {i === selectedIndex ? '❯ ' : '  '}
                {(stream.prefix ?? stream.stream_date ?? stream.id).padEnd(14)}
              </Text>
              <Text dimColor>{stream.title ?? '(no title)'}</Text>
            </Box>
          ))}
        </Box>
      )}

      <Box marginTop={1}>
        <Text dimColor>[↑/↓] navigate [Esc] back</Text>
      </Box>
    </Box>
  );
}
