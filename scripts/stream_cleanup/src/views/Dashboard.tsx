import { Box, Text, useInput } from 'ink';
import SelectInput from 'ink-select-input';
import React from 'react';
import { Spinner } from '../components/Spinner.js';
import { StatusBadge } from '../components/StatusBadge.js';
import type { PendingChange, ScanProgress } from '../services/types.js';
import type { ReconcileResult } from '../utils/reconcile.js';

type DashboardView =
  | 'incomplete-streams'
  | 'orphaned-files'
  | 'orphaned-streams'
  | 'count-mismatches'
  | 'dry-run-summary'
  | 'exit';

interface DashboardProps {
  reconcileResult: ReconcileResult;
  streamsScan: ScanProgress;
  s3Scan: ScanProgress;
  pendingChanges: PendingChange[];
  onSelectView: (view: DashboardView) => void;
}

interface MenuItem {
  label: string;
  value: DashboardView;
}

function scanningLabel(count: number, complete: boolean): string {
  if (complete) return String(count);
  return `${count}+`;
}

export function Dashboard({
  reconcileResult,
  streamsScan,
  s3Scan,
  pendingChanges,
  onSelectView,
}: DashboardProps) {
  const {
    incompleteStreams,
    orphanedS3Dates,
    orphanedStreams,
  } = reconcileResult;

  const streamsScanning = streamsScan.status === 'scanning';
  const s3Scanning = s3Scan.status === 'scanning';
  const eitherScanning = streamsScanning || s3Scanning;

  const menuItems: MenuItem[] = [
    {
      label: `Incomplete Streams (${scanningLabel(incompleteStreams.length, !streamsScanning)})`,
      value: 'incomplete-streams',
    },
    {
      label:
        streamsScan.status !== 'complete'
          ? 'Orphaned S3 Files  (waiting for DynamoDB scan…)'
          : `Orphaned S3 Files  (${orphanedS3Dates.length})`,
      value: 'orphaned-files',
    },
    {
      label:
        s3Scan.status !== 'complete'
          ? 'Orphaned Streams   (waiting for S3 scan…)'
          : `Orphaned Streams   (${orphanedStreams.length})`,
      value: 'orphaned-streams',
    },
    {
      label: `Pending Changes    (${pendingChanges.length})`,
      value: 'dry-run-summary',
    },
    { label: 'Exit', value: 'exit' },
  ];

  const handleSelect = (item: MenuItem) => {
    onSelectView(item.value);
  };

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Title */}
      <Box marginBottom={1}>
        <Text bold color="cyan">
          🧹 Stream Cleanup
        </Text>
      </Box>

      {/* Scan progress */}
      <Box flexDirection="column" marginBottom={1}>
        <Box gap={2}>
          <Text>DynamoDB:</Text>
          <Text>
            {streamsScan.itemsScanned.toLocaleString()} streams scanned
          </Text>
          {streamsScanning ? (
            <Spinner />
          ) : (
            <StatusBadge status={streamsScan.status} />
          )}
        </Box>
        <Box gap={2}>
          <Text>S3 bucket:</Text>
          <Text>{s3Scan.itemsScanned.toLocaleString()} objects scanned</Text>
          {s3Scanning ? <Spinner /> : <StatusBadge status={s3Scan.status} />}
        </Box>
      </Box>

      {/* Scan-in-progress notice */}
      {eitherScanning && (
        <Box marginBottom={1}>
          <Text color="yellow">
            ⚠ Scan in progress — some issue counts are preliminary.
            {'\n'} You can browse and queue fixes now; changes cannot be
            {'\n'} applied until scanning is complete.
          </Text>
        </Box>
      )}

      {/* Error notices */}
      {streamsScan.status === 'error' && (
        <Box marginBottom={1}>
          <Text color="red">✗ DynamoDB scan error: {streamsScan.error}</Text>
        </Box>
      )}
      {s3Scan.status === 'error' && (
        <Box marginBottom={1}>
          <Text color="red">✗ S3 scan error: {s3Scan.error}</Text>
        </Box>
      )}

      {/* Menu */}
      <SelectInput items={menuItems} onSelect={handleSelect} />
    </Box>
  );
}
