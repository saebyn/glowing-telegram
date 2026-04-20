import { Text } from 'ink';
import InkSpinner from 'ink-spinner';
import React from 'react';

interface SpinnerProps {
  label?: string;
}

export function Spinner({ label }: SpinnerProps) {
  return (
    <Text>
      <InkSpinner type="dots" />
      {label ? ` ${label}` : null}
    </Text>
  );
}
