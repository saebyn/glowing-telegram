# S3 Import Script

This script scans S3 to create basic metadata and stream records for older VOD segments without reading object contents (to avoid Glacier retrieval costs).

## Purpose

The script addresses the issue of older stream VOD segments in S3 that were never linked to the original stream records. It assumes all videos from one date are part of one stream and creates the necessary DynamoDB records.

## Prerequisites

- AWS credentials configured (via AWS CLI, environment variables, or IAM role)
- Access to the `saebyn-video-archive` S3 bucket
- Access to DynamoDB tables:
  - `streams-963700c` (streams table)
  - `metadata-table-aa16405` (video metadata table)
- Python 3.11+

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/glowing-telegram.git
   cd glowing-telegram/scripts/s3_import
   ```

2. Install dependencies:
   ```bash
  pip install -r requirements.txt
   ```

## Usage

```bash
# Basic usage - process all objects
python3 scripts/s3_import.py

# Dry run mode (recommended first)
python3 scripts/s3_import.py --dry-run

# Process only objects from a specific date
python3 scripts/s3_import.py --date 2023-08-31

# Process only objects with a specific prefix (e.g., all of August 2023)
python3 scripts/s3_import.py --prefix 2023-08

# Enable verbose logging
python3 scripts/s3_import.py --verbose --dry-run
```

## What the Script Does

1. **Lists S3 objects** matching the expected pattern: `date/date time.ext` (e.g., `2023-08-31/2023-08-31 16-42-55.mkv`)
2. **Groups objects by date** (assuming all videos from one date are part of one stream)
3. **Checks for existing stream records** for each date
4. **Creates stream records** if they don't exist for a given date
5. **Creates video clip records** linking each S3 object to its stream

## Safety Features

- **Dry run mode**: Test what would happen without making changes
- **Existing record detection**: Won't overwrite existing streams or video clips
- **Validation**: Strict parsing of S3 key patterns with date/time validation
- **Error handling**: Graceful handling of AWS errors with detailed logging
- **Atomic operations**: Each record creation is independent

## Expected S3 Key Format

The script expects S3 keys in this format:
```
YYYY-MM-DD/YYYY-MM-DD HH-MM-SS.extension
```

Examples:
- `2023-08-31/2023-08-31 16-42-55.mkv`
- `2023-12-01/2023-12-01 09-15-30.mp4`

Keys not matching this pattern will be skipped.

## Created Records

### Stream Records
- **ID**: Auto-generated UUID
- **Title**: "Stream YYYY-MM-DD"
- **Description**: "Imported stream from S3 for date YYYY-MM-DD"
- **Prefix**: "YYYY-MM-DD/"
- **Stream Date**: YYYY-MM-DD
- **Platform**: "twitch"
- **Video Clip Count**: Number of videos for that date
- **Timestamps**: Current ISO timestamp

### Video Clip Records
- **Key**: Original S3 key
- **Stream ID**: Associated stream ID
- **Note**: Start time is not set (would be set during normal ingestion)

## Limitations

- Does not read S3 object contents (by design, to avoid Glacier costs)
- Does not set video start times (would require object analysis)
- Assumes all videos from one date belong to one stream
- Does not validate that S3 objects actually exist or are accessible

## Error Handling

The script will:
- Log warnings for unrecognized key patterns
- Continue processing other objects if one fails
- Provide detailed error messages for AWS API failures
- Exit with non-zero code on fatal errors

## Examples

```bash
# Safe first run - see what would happen
python3 scripts/s3_import.py --dry-run --verbose

# Import only August 2023 videos
python3 scripts/s3_import.py --prefix 2023-08

# Import specific date
python3 scripts/s3_import.py --date 2023-08-31

# Full import (use carefully)
python3 scripts/s3_import.py
```