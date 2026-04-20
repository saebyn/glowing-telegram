import { Text } from 'ink';
import React from 'react';
import type { ScanStatus } from '../services/types.js';

interface StatusBadgeProps {
  status: ScanStatus;
}

const STATUS_CONFIG: Record<ScanStatus, { label: string; color: string }> = {
  idle: { label: 'idle', color: 'gray' },
  scanning: { label: 'scanning…', color: 'yellow' },
  complete: { label: '✓ done', color: 'green' },
  error: { label: '✗ error', color: 'red' },
};

export function StatusBadge({ status }: StatusBadgeProps) {
  const { label, color } = STATUS_CONFIG[status];
  return <Text color={color}>[{label}]</Text>;
}
