# Stream Cleanup Tool

An interactive CLI tool for auditing and repairing stream data across DynamoDB and S3.

## Purpose

Over time stream records in DynamoDB can become incomplete (missing `series_id`, `title`, etc.) or fall out of sync with the video files stored in S3. This tool:

- Scans the DynamoDB streams table and the S3 video archive bucket **concurrently and progressively** — UI updates are shown after every page, so you can start reviewing issues before the scan finishes.
- Cross-references the two data sources to surface orphaned S3 date-prefixes (files with no stream record) and orphaned stream records (records with no S3 files).
- Lets you queue fixes interactively and apply them in a single confirmed batch.

## Prerequisites

- Node.js 20+
- AWS credentials configured (AWS CLI, environment variables, or IAM role)
- Read access to the `saebyn-video-archive` S3 bucket
- Read/write access to DynamoDB:
  - `streams-963700c` (streams table)
  - `metadata-table-aa16405` (video metadata table)
  - *(optional)* a series table for the fuzzy series picker

## Installation

```bash
cd scripts/stream_cleanup
npm install
```

## Usage

```bash
# Recommended first run — see what would happen without making changes
npm start -- --dry-run

# Full interactive scan
npm start

# Restrict to a specific date/month prefix
npm start -- --prefix 2024-01

# Enable the series picker (fuzzy search)
npm start -- --series-table my-series-table

# Specify a different AWS region
npm start -- --region us-east-1
```

All CLI options:

| Flag | Default | Description |
|---|---|---|
| `--dry-run` | false | Show changes without writing to DynamoDB |
| `--bucket` | `saebyn-video-archive` | S3 bucket to scan |
| `--streams-table` | `streams-963700c` | DynamoDB streams table |
| `--metadata-table` | `metadata-table-aa16405` | DynamoDB video metadata table |
| `--series-table` | *(none)* | DynamoDB series table (enables fuzzy series picker) |
| `--prefix` | *(none)* | Scan only S3 objects with this key prefix |
| `--region` | *(from env)* | AWS region override |

## UI Flow

### Dashboard

On launch both scans start immediately in parallel. The dashboard shows live progress:

```
🧹 Stream Cleanup

  DynamoDB: 87 streams scanned  [scanning…]
  S3 bucket: 234 objects scanned  [scanning…]

⚠ Scan in progress — some issue counts are preliminary.
  You can browse and queue fixes now; changes cannot be
  applied until scanning is complete.

❯ Incomplete Streams (12+)
  Orphaned S3 Files  (waiting for DynamoDB scan…)
  Orphaned Streams   (waiting for S3 scan…)
  Count Mismatches   (waiting for both scans…)
  Pending Changes    (0)
  Exit
```

Counts update automatically as pages arrive. **Cross-referencing results
(orphaned files/streams, count mismatches) are intentionally suppressed until
the relevant scan completes** to prevent false positives.

### Incomplete Streams

Shows streams missing one or more required fields (`series_id`, `stream_date`,
`title`, `stream_platform`). The list grows as the DynamoDB scan progresses.

Select a stream and press `Enter` to open the detail editor.

### Stream Detail

Edit individual stream fields. For `series_id`, a fuzzy search picker is shown
(requires `--series-table` to be configured).

Changes are **queued** in memory, not written immediately. If a change is queued
while scanning is still in progress it is flagged as **pre-scan** and must be
re-reviewed before it can be applied.

### Orphaned S3 Files

S3 date-prefixes that have video files but no matching stream record. Only
shown after the DynamoDB scan completes (to avoid false positives).

Press `Enter` on a date to queue a "create stream record" change.

### Dry-Run Summary

Shows all queued changes before any writes. Apply is **blocked** until:

1. Both the DynamoDB and S3 scans are complete.
2. Any **invalidated** changes (e.g. a "create stream" change for a date where
   a stream record was later discovered) have been discarded.

Changes flagged **pre-scan** (queued before scan completion) are highlighted
with `⚠` and require manual verification.

```
Pending Changes

⚠ 2 changes were queued before scanning completed.
  Please review them before applying.

❯ ⚠ Set series_id on stream 2023-09-14  [pre-scan]
  ✓ Create stream record for 2023-10-01 (8 files)

[a] Apply all   [d] Discard selected   [Esc] Back
```

## Safety Features

- **No writes until confirmed** — all changes are staged in a pending queue.
- **Scan-complete gate** — changes cannot be applied while either scan is running.
- **Invalidation detection** — a "create stream" change is automatically flagged
  as invalid if the completed scan reveals a stream record already exists for that date.
- **Dry-run mode** — `--dry-run` simulates the apply step with no DynamoDB writes.
- **No S3 content reads** — avoids Glacier retrieval costs; only object metadata
  (key, size, last-modified) is inspected.

## Expected S3 Key Format

```
YYYY-MM-DD/YYYY-MM-DD HH-MM-SS.extension
```

Examples:
- `2023-08-31/2023-08-31 16-42-55.mkv`
- `2024-01-15/2024-01-15 09-30-00.mp4`

Keys that don't match this pattern are silently skipped.
