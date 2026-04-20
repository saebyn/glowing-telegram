import { Box, Text } from 'ink';
import React from 'react';
import type { Stream } from '../services/types.js';

interface StreamTableProps {
  streams: Array<{ stream: Stream; missingFields: string[] }>;
  selectedIndex: number;
  queuedStreamIds?: Set<string>;
  /** Maximum number of rows to render (used to implement scrolling). */
  maxRows?: number;
}

/** Truncate a string to at most `max` characters, appending "…" if cut. */
function trunc(s: string | undefined, max: number): string {
  if (!s) return '';
  return s.length <= max ? s : `${s.slice(0, max - 1)}…`;
}

export function StreamTable({
  streams,
  selectedIndex,
  queuedStreamIds,
  maxRows = 20,
}: StreamTableProps) {
  // Compute visible window for scrolling.
  const half = Math.floor(maxRows / 2);
  const start = Math.max(
    0,
    Math.min(selectedIndex - half, streams.length - maxRows),
  );
  const visible = streams.slice(start, start + maxRows);

  return (
    <Box flexDirection="column">
      {/* Header */}
      <Box>
        <Text bold>{'  '}</Text>
        <Text bold color="cyan">
          {'Date        '}
        </Text>
        <Text bold color="cyan">
          {'Title                    '}
        </Text>
        <Text bold color="cyan">
          {'Missing fields      Status'}
        </Text>
      </Box>
      <Box>
        <Text dimColor>
          {
            '  ──────────  ─────────────────────────  ──────────────────  ──────'
          }
        </Text>
      </Box>

      {visible.map((item, i) => {
        const absoluteIndex = start + i;
        const isSelected = absoluteIndex === selectedIndex;
        const date =
          item.stream.stream_date ??
          item.stream.prefix?.replace(/\/$/, '') ??
          '???';
        const isQueued = queuedStreamIds?.has(item.stream.id) ?? false;
        return (
          <Box key={item.stream.id}>
            <Text color={isSelected ? 'cyan' : undefined}>
              {isSelected ? '❯ ' : '  '}
            </Text>
            <Text color={isSelected ? 'cyan' : undefined}>
              {`${trunc(date, 10).padEnd(12)}`}
            </Text>
            <Text color={isSelected ? 'cyan' : undefined}>
              {`${trunc(item.stream.title, 25).padEnd(27)}`}
            </Text>
            <Text color="yellow">{item.missingFields.join(', ')}</Text>
            {isQueued && <Text color="green"> queued</Text>}
          </Box>
        );
      })}

      {streams.length > maxRows && (
        <Text dimColor>
          {`  … ${streams.length - maxRows} more (scroll with ↑/↓)`}
        </Text>
      )}
    </Box>
  );
}
