import { Box, Text, useInput } from 'ink';
import React, { useState } from 'react';
import { Spinner } from '../components/Spinner.js';
import type { S3VideoObject, ScanProgress } from '../services/types.js';
import type { ReconcileResult } from '../utils/reconcile.js';

interface OrphanedFilesProps {
  reconcileResult: ReconcileResult;
  streamsScan: ScanProgress;
  s3Scan: ScanProgress;
  bothScansComplete: boolean;
  onCreateStream: (date: string, files: S3VideoObject[]) => void;
  onBack: () => void;
}

export function OrphanedFiles({
  reconcileResult,
  streamsScan,
  s3Scan,
  bothScansComplete,
  onCreateStream,
  onBack,
}: OrphanedFilesProps) {
  const { orphanedS3Dates } = reconcileResult;
  const [selectedIndex, setSelectedIndex] = useState(0);

  const streamsScanning = streamsScan.status === 'scanning';
  const s3Scanning = s3Scan.status === 'scanning';

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
      setSelectedIndex((i) => Math.min(orphanedS3Dates.length - 1, i + 1));
      return;
    }
    if (key.return && orphanedS3Dates.length > 0) {
      const item = orphanedS3Dates[selectedIndex];
      if (item) onCreateStream(item.date, item.files);
    }
  });

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Header */}
      <Box marginBottom={1} gap={2}>
        <Text bold color="cyan">
          Orphaned S3 Files
        </Text>
        {s3Scanning && <Spinner />}
      </Box>

      {/* Explain that results aren't ready until DynamoDB is done */}
      {streamsScanning && (
        <Box marginBottom={1}>
          <Text color="yellow">
            ⚠ DynamoDB scan still running — results shown below may not be
            {'\n'} orphaned; a matching stream record might not yet be
            {'\n'} loaded. This list will update automatically.
          </Text>
        </Box>
      )}

      {/* Results */}
      {!streamsScanning && orphanedS3Dates.length === 0 ? (
        <Text color="green">✓ No orphaned S3 date-prefixes found.</Text>
      ) : orphanedS3Dates.length === 0 ? (
        <Text dimColor>
          Waiting for DynamoDB scan to finish before showing results…
        </Text>
      ) : (
        <Box flexDirection="column">
          {orphanedS3Dates.map((item, i) => (
            <Box key={item.date}>
              <Text color={i === selectedIndex ? 'cyan' : undefined}>
                {i === selectedIndex ? '❯ ' : '  '}
                {item.date.padEnd(14)}
              </Text>
              <Text dimColor>{item.files.length} file(s)</Text>
            </Box>
          ))}
        </Box>
      )}

      {/* Cannot create streams until both scans complete */}
      {!bothScansComplete && orphanedS3Dates.length > 0 && (
        <Box marginTop={1}>
          <Text color="yellow">
            ⚠ Cannot queue "create stream" until both scans complete.
          </Text>
        </Box>
      )}

      <Box marginTop={1}>
        <Text dimColor>
          {bothScansComplete
            ? '[↑/↓] navigate  [Enter] create stream record  [Esc] back'
            : '[↑/↓] navigate  [Esc] back  (Enter disabled until scan complete)'}
        </Text>
      </Box>
    </Box>
  );
}
