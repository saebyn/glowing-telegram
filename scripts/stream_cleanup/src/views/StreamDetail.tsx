import { Box, Text, useInput } from 'ink';
import TextInput from 'ink-text-input';
import React, { useState } from 'react';
import { SelectSeries } from '../components/SelectSeries.js';
import type { PendingChangeInput, Series, Stream } from '../services/types.js';

type Field =
  | 'series_id'
  | 'stream_date'
  | 'title'
  | 'stream_platform'
  | 'description';

interface StreamDetailProps {
  stream: Stream;
  series: Series[];
  bothScansComplete: boolean;
  onQueueChange: (change: PendingChangeInput) => void;
  onBack: () => void;
}

/**
 * Simple editing state — `mode` discriminates between no edit, picking a
 * series, and inline text input.  Using a separate `mode` field avoids
 * TypeScript narrowing false-positives on the `field` property.
 */
type EditingState =
  | { mode: 'none' }
  | { mode: 'series' }
  | { mode: 'text'; field: Exclude<Field, 'series_id'>; value: string };

export function StreamDetail({
  stream,
  series,
  bothScansComplete,
  onQueueChange,
  onBack,
}: StreamDetailProps) {
  const [editing, setEditing] = useState<EditingState>({ mode: 'none' });
  const [selectedFieldIndex, setSelectedFieldIndex] = useState(0);

  const displayDate =
    stream.stream_date ?? stream.prefix?.replace(/\/$/, '') ?? '(no date)';

  const editableFields: Array<{ key: Field; label: string; current: string }> =
    [
      { key: 'title', label: 'Title', current: stream.title ?? '' },
      {
        key: 'stream_date',
        label: 'Stream Date',
        current: stream.stream_date ?? '',
      },
      {
        key: 'stream_platform',
        label: 'Platform',
        current: stream.stream_platform ?? 'twitch',
      },
      { key: 'series_id', label: 'Series', current: stream.series_id ?? '' },
      {
        key: 'description',
        label: 'Description',
        current: stream.description ?? '',
      },
    ];

  useInput((_input, key) => {
    if (editing.mode !== 'none') return; // Let child components handle input.

    if (key.escape) {
      onBack();
      return;
    }
    if (key.upArrow) {
      setSelectedFieldIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedFieldIndex((i) => Math.min(editableFields.length - 1, i + 1));
      return;
    }
    if (key.return) {
      const field = editableFields[selectedFieldIndex];
      if (!field) return;
      if (field.key === 'series_id') {
        setEditing({ mode: 'series' });
      } else {
        setEditing({ mode: 'text', field: field.key, value: field.current });
      }
    }
  });

  const handleTextSubmit = (value: string) => {
    if (editing.mode !== 'text') return;
    const field = editing.field;
    const trimmed = value.trim();
    if (trimmed) {
      onQueueChange({
        type: 'update_stream',
        description: `Set ${field} = "${trimmed}" on stream ${displayDate}`,
        streamId: stream.id,
        updates: { [field]: trimmed },
      });
    }
    setEditing({ mode: 'none' });
  };

  const handleSeriesSelect = (selected: Series) => {
    onQueueChange({
      type: 'update_stream',
      description: `Set series_id = "${selected.id}" (${selected.title}) on stream ${displayDate}`,
      streamId: stream.id,
      updates: { series_id: selected.id },
    });
    setEditing({ mode: 'none' });
  };

  if (editing.mode === 'series') {
    return (
      <Box flexDirection="column" paddingX={1}>
        <Box marginBottom={1}>
          <Text bold color="cyan">
            Pick Series — {displayDate}
          </Text>
        </Box>
        <SelectSeries
          series={series}
          onSelect={handleSeriesSelect}
          onCancel={() => setEditing({ mode: 'none' })}
        />
      </Box>
    );
  }

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Header */}
      <Box marginBottom={1}>
        <Text bold color="cyan">
          Edit Stream — {displayDate}
        </Text>
        {!bothScansComplete && <Text color="yellow"> ⚠ pre-scan</Text>}
      </Box>

      {!bothScansComplete && (
        <Box marginBottom={1}>
          <Text color="yellow">
            ⚠ Scans are still running. Changes queued now will be flagged
            {'\n'} for re-review in the summary before they can be applied.
          </Text>
        </Box>
      )}

      {/* Field list */}
      {editableFields.map((f, i) => {
        const isSelected = i === selectedFieldIndex;
        const isInlineEdit = editing.mode === 'text' && editing.field === f.key;

        return (
          <Box key={f.key} marginBottom={isInlineEdit ? 1 : 0}>
            <Text color={isSelected ? 'cyan' : undefined}>
              {isSelected ? '❯ ' : '  '}
              {`${f.label.padEnd(14)} `}
            </Text>
            {isInlineEdit ? (
              <TextInput
                value={editing.value}
                onChange={(v) =>
                  setEditing({
                    mode: 'text',
                    field: editing.field,
                    value: v,
                  })
                }
                onSubmit={handleTextSubmit}
              />
            ) : (
              <Text dimColor={!f.current}>{f.current || '(not set)'}</Text>
            )}
          </Box>
        );
      })}

      <Box marginTop={1}>
        <Text dimColor>
          [↑/↓] navigate [Enter] edit field [Esc] back to list
        </Text>
      </Box>
    </Box>
  );
}
