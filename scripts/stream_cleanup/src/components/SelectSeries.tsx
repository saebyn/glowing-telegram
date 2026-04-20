import Fuse from 'fuse.js';
import { Box, Text, useInput } from 'ink';
import TextInput from 'ink-text-input';
import React, { useState, useCallback } from 'react';
import type { Series } from '../services/types.js';

interface SelectSeriesProps {
  series: Series[];
  onSelect: (series: Series) => void;
  onCancel: () => void;
}

export function SelectSeries({
  series,
  onSelect,
  onCancel,
}: SelectSeriesProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);

  const fuse = new Fuse(series, {
    keys: ['title', 'description'],
    threshold: 0.4,
  });

  const results: Series[] =
    query.trim() === ''
      ? series.slice(0, 10)
      : fuse
          .search(query)
          .map((r) => r.item)
          .slice(0, 10);

  const handleQueryChange = useCallback((value: string) => {
    setQuery(value);
    setSelectedIndex(0);
  }, []);

  useInput((input, key) => {
    if (key.escape) {
      onCancel();
      return;
    }
    if (key.upArrow) {
      setSelectedIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIndex((i) => Math.min(results.length - 1, i + 1));
      return;
    }
    if (key.return) {
      const item = results[selectedIndex];
      if (item) onSelect(item);
    }
  });

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text bold>Search series: </Text>
        <TextInput
          value={query}
          onChange={handleQueryChange}
          placeholder="type to filter…"
        />
      </Box>

      {series.length === 0 ? (
        <Text dimColor>
          No series available. Pass --series-table to enable series lookup.
        </Text>
      ) : results.length === 0 ? (
        <Text dimColor>No results for "{query}"</Text>
      ) : (
        results.map((s, i) => (
          <Box key={s.id}>
            <Text color={i === selectedIndex ? 'cyan' : undefined}>
              {i === selectedIndex ? '❯ ' : '  '}
              {s.title}
            </Text>
          </Box>
        ))
      )}

      <Box marginTop={1}>
        <Text dimColor>[↑/↓] navigate [Enter] select [Esc] cancel</Text>
      </Box>
    </Box>
  );
}
