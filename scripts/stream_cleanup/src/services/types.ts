/**
 * Local type definitions for stream-cleanup.
 *
 * Stream and Series interfaces are re-declared here to avoid fragile
 * cross-workspace relative imports from types/src/types.ts. They must
 * stay in sync with the canonical definitions there.
 */

// ---------- Domain types (mirror of types/src/types.ts) ----------

export interface Stream {
  created_at?: string;
  description?: string;
  duration?: number;
  has_episodes?: boolean;
  id: string;
  prefix?: string;
  series_id?: string;
  stream_date?: string;
  stream_platform?: string;
  thumbnail_url?: string;
  title?: string;
  updated_at?: string;
  video_clip_count?: number;
}

export interface Series {
  id: string;
  title: string;
  description?: string;
  created_at: string;
  updated_at?: string;
}

// ---------- S3 ----------

export interface S3VideoObject {
  key: string;
  /** YYYY-MM-DD extracted from the S3 key prefix */
  date: string;
  /** Filename portion after the date prefix */
  filename: string;
  size?: number;
  lastModified?: Date;
}

// ---------- Scan progress ----------

export type ScanStatus = 'idle' | 'scanning' | 'complete' | 'error';

export interface ScanProgress {
  status: ScanStatus;
  pagesScanned: number;
  itemsScanned: number;
  error?: string;
}

// ---------- Pending changes ----------

export interface UpdateStreamChange {
  id: string;
  type: 'update_stream';
  description: string;
  streamId: string;
  updates: Partial<Stream>;
  /**
   * True when both the DynamoDB and S3 scans were complete at the time this
   * change was queued. False means the results that prompted this change may
   * be incomplete; the user should re-review before applying.
   */
  createdWithScansComplete: boolean;
}

export interface CreateStreamChange {
  id: string;
  type: 'create_stream';
  description: string;
  date: string;
  stream_date?: string;
  files: S3VideoObject[];
  createdWithScansComplete: boolean;
}

export interface UpdateCountChange {
  id: string;
  type: 'update_count';
  description: string;
  streamId: string;
  newCount: number;
  createdWithScansComplete: boolean;
}

export type PendingChange =
  | UpdateStreamChange
  | CreateStreamChange
  | UpdateCountChange;

/**
 * Union of the per-change input shapes (without `id` and
 * `createdWithScansComplete`, which are added by the queue helper).
 */
export type PendingChangeInput =
  | Omit<UpdateStreamChange, 'id' | 'createdWithScansComplete'>
  | Omit<CreateStreamChange, 'id' | 'createdWithScansComplete'>
  | Omit<UpdateCountChange, 'id' | 'createdWithScansComplete'>;
