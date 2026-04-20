export interface Config {
  bucket: string;
  streamsTable: string;
  metadataTable: string;
  seriesTable: string;
  region?: string;
  dryRun: boolean;
  prefix?: string;
}

export const DEFAULT_CONFIG: Config = {
  bucket: 'saebyn-video-archive',
  streamsTable: 'streams-963700c',
  metadataTable: 'metadata-table-aa16405',
  seriesTable: 'stream-series-09d6bad',
  dryRun: false,
};
