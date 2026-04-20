import { Box, Text, useInput } from 'ink';
import React, { useState } from 'react';
import { Confirm } from '../components/Confirm.js';
import type { PendingChange, Stream } from '../services/types.js';
import { getStreamDateKey } from '../utils/reconcile.js';

interface DryRunSummaryProps {
  pendingChanges: PendingChange[];
  streams: Stream[];
  bothScansComplete: boolean;
  dryRun: boolean;
  onApply: () => void;
  onDiscard: (changeId: string) => void;
  onBack: () => void;
}

/**
 * Check whether a create_stream change is still valid.
 * If the scan is now complete and a stream already covers the date,
 * the change has been invalidated.
 */
function isCreateStreamInvalidated(
  change: Extract<PendingChange, { type: 'create_stream' }>,
  streams: Stream[],
  bothScansComplete: boolean,
): boolean {
  if (!bothScansComplete) return false; // Can't tell yet.
  return streams.some((s) => getStreamDateKey(s) === change.date);
}

export function DryRunSummary({
  pendingChanges,
  streams,
  bothScansComplete,
  dryRun,
  onApply,
  onDiscard,
  onBack,
}: DryRunSummaryProps) {
  const [confirming, setConfirming] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);

  const preScanChanges = pendingChanges.filter(
    (c) => !c.createdWithScansComplete,
  );
  const invalidatedChanges = pendingChanges.filter(
    (c) =>
      c.type === 'create_stream' &&
      isCreateStreamInvalidated(
        c as Extract<PendingChange, { type: 'create_stream' }>,
        streams,
        bothScansComplete,
      ),
  );

  const canApply =
    bothScansComplete &&
    pendingChanges.length > 0 &&
    invalidatedChanges.length === 0;

  useInput((_input, key) => {
    if (confirming) return;

    if (key.escape) {
      onBack();
      return;
    }
    if (key.upArrow) {
      setSelectedIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIndex((i) => Math.min(pendingChanges.length - 1, i + 1));
      return;
    }
    if (_input === 'd' || _input === 'D') {
      const change = pendingChanges[selectedIndex];
      if (change) onDiscard(change.id);
      setSelectedIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (_input === 'a' || _input === 'A') {
      if (canApply) setConfirming(true);
      return;
    }
  });

  if (confirming) {
    return (
      <Box paddingX={1} flexDirection="column">
        <Confirm
          message={
            dryRun
              ? `Simulate applying ${pendingChanges.length} change(s)? (dry-run mode — no writes)`
              : `Apply ${pendingChanges.length} change(s) to DynamoDB? This cannot be undone.`
          }
          onConfirm={() => {
            setConfirming(false);
            onApply();
          }}
          onCancel={() => setConfirming(false)}
        />
      </Box>
    );
  }

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Title */}
      <Box marginBottom={1}>
        <Text bold color="cyan">
          Pending Changes
        </Text>
        {dryRun && (
          <Text color="yellow"> (dry-run — no writes will occur)</Text>
        )}
      </Box>

      {/* Scan-not-complete block */}
      {!bothScansComplete && (
        <Box marginBottom={1}>
          <Text color="red" bold>
            ⛔ Scan not complete — changes cannot be applied yet.
          </Text>
          <Text color="yellow">
            {'\n'} Wait for both DynamoDB and S3 scans to finish before
            {'\n'} committing any changes.
          </Text>
        </Box>
      )}

      {/* Invalidated changes warning */}
      {invalidatedChanges.length > 0 && (
        <Box marginBottom={1}>
          <Text color="red">
            ✗ {invalidatedChanges.length} change(s) are now invalid because
            {'\n'} a matching stream was found during the scan. Please
            {'\n'} discard them [d] before applying.
          </Text>
        </Box>
      )}

      {/* Pre-scan changes warning */}
      {bothScansComplete &&
        preScanChanges.length > 0 &&
        invalidatedChanges.length === 0 && (
          <Box marginBottom={1}>
            <Text color="yellow">
              ⚠ {preScanChanges.length} change(s) were queued before scanning
              {'\n'} completed. Please verify they are still correct.
            </Text>
          </Box>
        )}

      {/* Change list */}
      {pendingChanges.length === 0 ? (
        <Text dimColor>No pending changes.</Text>
      ) : (
        pendingChanges.map((change, i) => {
          const isSelected = i === selectedIndex;
          const isPreScan = !change.createdWithScansComplete;
          const isInvalid =
            change.type === 'create_stream' &&
            isCreateStreamInvalidated(
              change as Extract<PendingChange, { type: 'create_stream' }>,
              streams,
              bothScansComplete,
            );

          return (
            <Box key={change.id}>
              <Text color={isSelected ? 'cyan' : undefined}>
                {isSelected ? '❯ ' : '  '}
              </Text>
              {isInvalid ? (
                <Text color="red">✗ </Text>
              ) : isPreScan ? (
                <Text color="yellow">⚠ </Text>
              ) : (
                <Text color="green">✓ </Text>
              )}
              <Text color={isSelected ? 'cyan' : isInvalid ? 'red' : undefined}>
                {change.description}
              </Text>
              {isPreScan && !isInvalid && (
                <Text color="yellow"> [pre-scan]</Text>
              )}
              {isInvalid && <Text color="red"> [invalidated]</Text>}
            </Box>
          );
        })
      )}

      {/* Controls */}
      <Box marginTop={1} flexDirection="column">
        {canApply ? (
          <Text>
            <Text bold color="green">
              [a]
            </Text>{' '}
            Apply all{'   '}
            <Text bold>[d]</Text> Discard selected{'   '}
            <Text bold>[Esc]</Text> Back
          </Text>
        ) : (
          <Text dimColor>
            [d] Discard selected [Esc] Back
            {!bothScansComplete && '  (apply locked until scan complete)'}
            {invalidatedChanges.length > 0 &&
              '  (apply locked — discard invalid changes first)'}
          </Text>
        )}
      </Box>
    </Box>
  );
}
