#!/usr/bin/env node
import { render } from 'ink';
import meow from 'meow';
import React from 'react';
import { App } from './app.js';
import { type Config, DEFAULT_CONFIG } from './utils/config.js';

const cli = meow(
  `
  Usage
    $ stream-cleanup [options]

  Options
    --dry-run               Show changes without applying them
    --bucket <name>         S3 bucket (default: ${DEFAULT_CONFIG.bucket})
    --streams-table <name>  DynamoDB streams table (default: ${DEFAULT_CONFIG.streamsTable})
    --metadata-table <name> DynamoDB video metadata table (default: ${DEFAULT_CONFIG.metadataTable})
    --series-table <name>   DynamoDB series table (enables series picker)
    --prefix <prefix>       Only scan S3 objects with this prefix
    --region <region>       AWS region (default: from environment/config)

  Examples
    $ stream-cleanup --dry-run
    $ stream-cleanup --series-table my-series-table
    $ stream-cleanup --prefix 2024-01 --region us-east-1
`,
  {
    importMeta: import.meta,
    flags: {
      dryRun: { type: 'boolean', default: false },
      bucket: { type: 'string', default: DEFAULT_CONFIG.bucket },
      streamsTable: { type: 'string', default: DEFAULT_CONFIG.streamsTable },
      metadataTable: { type: 'string', default: DEFAULT_CONFIG.metadataTable },
      seriesTable: { type: 'string' },
      prefix: { type: 'string' },
      region: { type: 'string' },
    },
  },
);

const config: Config = {
  dryRun: cli.flags.dryRun,
  bucket: cli.flags.bucket,
  streamsTable: cli.flags.streamsTable,
  metadataTable: cli.flags.metadataTable,
  seriesTable: cli.flags.seriesTable,
  prefix: cli.flags.prefix,
  region: cli.flags.region,
};

render(<App config={config} />);
