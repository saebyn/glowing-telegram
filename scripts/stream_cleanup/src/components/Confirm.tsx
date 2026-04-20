import { Box, Text, useInput } from 'ink';
import React from 'react';

interface ConfirmProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function Confirm({ message, onConfirm, onCancel }: ConfirmProps) {
  useInput((input, key) => {
    if (input === 'y' || input === 'Y') onConfirm();
    if (input === 'n' || input === 'N' || key.escape) onCancel();
  });

  return (
    <Box flexDirection="column">
      <Text>{message}</Text>
      <Box marginTop={1}>
        <Text>
          <Text color="green" bold>
            [y]
          </Text>{' '}
          Yes {'  '}
          <Text color="red" bold>
            [n/Esc]
          </Text>{' '}
          No
        </Text>
      </Box>
    </Box>
  );
}
