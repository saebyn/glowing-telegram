#!/usr/bin/env python3
"""
Script to scan S3 and create basic metadata and stream records for older VOD segments.

This script:
- Lists S3 objects without reading their contents (to avoid Glacier retrieval costs)
- Groups objects by date (assuming all videos from one date are part of one stream)
- Creates stream records if they don't already exist for a given date
- Creates video_clip records that link to each S3 object

Key structure expected: "date/date time.ext" (e.g., "2023-08-31/2023-08-31 16-42-55.mkv")

Usage:
    python3 scripts/s3_import.py [--dry-run] [--prefix PREFIX] [--date DATE]

Arguments:
    --dry-run    Don't actually create records, just show what would be done
    --prefix     Only process objects with this prefix (e.g., "2023-08")
    --date       Only process objects for this specific date (YYYY-MM-DD)
"""

import argparse
import boto3
from boto3.dynamodb.conditions import Attr
import logging
import re
import uuid
from datetime import datetime
from collections import defaultdict
from typing import Dict, List, Optional, TypedDict

from mypy_boto3_dynamodb import DynamoDBServiceResource
from mypy_boto3_dynamodb.service_resource import Table
from mypy_boto3_s3 import S3Client

logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Default configuration for S3 bucket and DynamoDB tables
DEFAULT_BUCKET_NAME = "saebyn-video-archive"
DEFAULT_STREAMS_TABLE = "streams-963700c"
DEFAULT_VIDEO_METADATA_TABLE = "metadata-table-aa16405"

# Pattern to match expected S3 key structure: date/date time.extension
# More strict pattern that validates date and time ranges
KEY_PATTERN = re.compile(
    r"^(\d{4}-\d{2}-\d{2})/(\d{4}-\d{2}-\d{2} \d{2}-\d{2}-\d{2}\.[\w]+)$"
)


def is_valid_date_time(date_str: str, time_str: str) -> bool:
    """Validate that the date and time components are realistic.

    Args:
        date_str: Date string in YYYY-MM-DD format
        time_str: Time string in HH-MM-SS format

    Returns:
        True if valid, False otherwise
    """
    try:
        # Parse and validate date
        year, month, day = map(int, date_str.split("-"))
        if not (1900 <= year <= 2100 and 1 <= month <= 12 and 1 <= day <= 31):
            return False

        # Parse and validate time
        hour, minute, second = map(int, time_str.split("-"))
        if not (0 <= hour <= 23 and 0 <= minute <= 59 and 0 <= second <= 59):
            return False

        return True
    except (ValueError, IndexError):
        return False


def parse_s3_key(key: str) -> Optional[tuple[str, str]]:
    """Parse S3 key to extract date and filename.

    Args:
        key: S3 object key

    Returns:
        Tuple of (date, filename) or None if key doesn't match expected pattern
    """
    match = KEY_PATTERN.match(key)
    if match:
        date_part = match.group(1)
        filename = match.group(2)

        # Extract time part from filename for validation
        # Format: "YYYY-MM-DD HH-MM-SS.ext"
        try:
            time_part = filename.split(" ")[1].split(".")[0]  # Extract "HH-MM-SS"
            if is_valid_date_time(date_part, time_part):
                return date_part, filename
        except (IndexError, ValueError):
            pass
    return None


class S3ImportError(Exception):
    """Custom exception for S3 import operations."""

    pass


class VideoObjects(TypedDict):
    """TypedDict for video object metadata."""

    key: str
    date: str
    filename: str
    size: int
    last_modified: Optional[datetime]


def list_video_objects(
    s3_client: S3Client, bucket_name: str, prefix: str = ""
) -> List[VideoObjects]:
    """List all video objects in the S3 bucket.

    Args:
        s3_client: Boto3 S3 client
        bucket_name: Name of the S3 bucket
        prefix: Optional prefix to filter objects

    Returns:
        List of objects with metadata
    """
    logger.info(f"📋 Listing objects in bucket {bucket_name} with prefix '{prefix}'...")

    objects: List[VideoObjects] = []
    paginator = s3_client.get_paginator("list_objects_v2")

    page_iterator = paginator.paginate(Bucket=bucket_name, Prefix=prefix)

    for page in page_iterator:
        if "Contents" in page:
            for obj in page["Contents"]:
                key = obj.get("Key")
                if not key:
                    logger.warning("Found object with no key, skipping")
                    continue
                parsed = parse_s3_key(key)
                if parsed:
                    date, filename = parsed
                    objects.append(
                        {
                            "key": key,
                            "date": date,
                            "filename": filename,
                            "size": obj.get("Size", 0),
                            "last_modified": obj.get("LastModified"),
                        }
                    )
                else:
                    logger.debug(f"Skipping object with non-matching key: {key}")

    logger.info(f"📦 Found {len(objects)} video objects matching the expected pattern")
    return objects


def group_objects_by_date(objects: List[VideoObjects]) -> Dict[str, List[VideoObjects]]:
    """Group objects by their date prefix.

    Args:
        objects: List of object metadata

    Returns:
        Dictionary mapping date to list of objects
    """
    grouped: Dict[str, List[VideoObjects]] = defaultdict(list)
    for obj in objects:
        grouped[obj["date"]].append(obj)

    logger.info(f"📅 Grouped objects into {len(grouped)} dates")
    return dict(grouped)


def stream_exists(streams_table: Table, stream_date: str) -> bool:
    """Check if a stream record already exists for the given date.

    Args:
        streams_table: DynamoDB table resource
        stream_date: Date string in YYYY-MM-DD format

    Returns:
        True if stream exists, False otherwise
    """
    try:
        # Query by stream_date to see if any stream exists for this date
        response = streams_table.scan(
            FilterExpression=Attr("stream_date").eq(stream_date),
            ProjectionExpression="id",
        )

        return len(response["Items"]) > 0
    except Exception as e:
        logger.error(f"Error checking if stream exists for date {stream_date}: {e}")
        return False


def create_stream_record(
    streams_table: Table,
    stream_date: str,
    video_count: int,
    dry_run: bool = False,
) -> str:
    """Create a new stream record for the given date.

    Args:
        streams_table: DynamoDB table resource for streams
        stream_date: Date string in YYYY-MM-DD format
        video_count: Number of video clips in this stream
        dry_run: If True, don't actually create the record

    Returns:
        The created stream ID
    """
    stream_id = str(uuid.uuid4())
    now = datetime.now().isoformat()

    if dry_run:
        logger.info(
            f"🔍 [DRY RUN] Would create stream record {stream_id} for date {stream_date}"
        )
        return stream_id

    try:
        streams_table.put_item(
            Item={
                "id": stream_id,
                "title": f"Stream {stream_date}",
                "description": f"Imported stream from S3 for date {stream_date}",
                "prefix": f"{stream_date}/",
                "created_at": now,
                "updated_at": now,
                "stream_date": stream_date,
                "stream_platform": "twitch",
                "video_clip_count": video_count,
                "has_episodes": False,
            }
        )
        logger.info(f"✨ Created stream record {stream_id} for date {stream_date}")
        return stream_id
    except Exception as e:
        logger.error(f"Error creating stream record for date {stream_date}: {e}")
        raise S3ImportError(f"Failed to create stream record: {e}")


def video_clip_exists(video_metadata_table: Table, key: str) -> bool:
    """Check if a video clip record already exists for the given S3 key.

    Args:
        video_metadata_table: DynamoDB table resource for video metadata
        key: S3 object key

    Returns:
        True if video clip exists, False otherwise
    """
    try:
        response = video_metadata_table.get_item(Key={"key": key})
        return "Item" in response
    except Exception as e:
        logger.error(f"Error checking if video clip exists for key {key}: {e}")
        return False


def create_video_clip_record(
    video_metadata_table: Table,
    obj: VideoObjects,
    stream_id: str,
    dry_run: bool = False,
):
    """Create a video clip record for the given S3 object.

    Args:
        dynamodb: Boto3 DynamoDB resource
        obj: Object metadata dictionary
        stream_id: ID of the associated stream
        dry_run: If True, don't actually create the record
    """
    if dry_run:
        logger.info(f"🔍 [DRY RUN] Would create video clip record for {obj['key']}")
        return

    try:
        video_metadata_table.put_item(
            Item={
                "key": obj["key"],
                "stream_id": stream_id,
                # Note: We're not setting start_time since we don't have that information
                # from the S3 metadata alone. This would typically be set during ingestion.
            }
        )
        logger.info(f"🎬 Created video clip record for {obj['key']}")
    except Exception as e:
        logger.error(f"Error creating video clip record for {obj['key']}: {e}")
        raise S3ImportError(f"Failed to create video clip record: {e}")


def process_date_group(
    streams_table: Table,
    video_metadata_table: Table,
    date: str,
    objects: List[VideoObjects],
    dry_run: bool = False,
) -> Optional[str]:
    """Process all objects for a single date.

    Args:
        streams_table: DynamoDB table resource for streams
        video_metadata_table: DynamoDB table resource for video metadata
        date: Date string in YYYY-MM-DD format
        objects: List of objects for this date
        dry_run: If True, don't actually create records

    Returns:
        Stream ID if processing succeeded, None otherwise
    """
    logger.info(f"🗓️ Processing {len(objects)} objects for date {date}")

    # Check if stream already exists for this date
    if stream_exists(streams_table, date):
        logger.info(
            f"⏭️ Stream already exists for date {date}, skipping stream creation"
        )
        # We still need to find the stream ID to create video clip records
        response = streams_table.scan(
            FilterExpression=Attr("stream_date").eq(date),
            ProjectionExpression="id",
        )
        if response["Items"]:
            stream_id = str(response["Items"][0]["id"])
        else:
            logger.error(
                f"Stream exists check returned true, but couldn't find stream for date {date}"
            )
            return None
    else:
        # Create new stream record
        stream_id = create_stream_record(streams_table, date, len(objects), dry_run)

    # Create video clip records for objects that don't already exist
    created_clips = 0
    skipped_clips = 0

    for obj in objects:
        if video_clip_exists(video_metadata_table, obj["key"]):
            logger.debug(f"Video clip already exists for {obj['key']}, skipping")
            skipped_clips += 1
        else:
            create_video_clip_record(video_metadata_table, obj, stream_id, dry_run)
            created_clips += 1

    if dry_run:
        logger.info(
            f"🔍 [DRY RUN] Date {date}: Would create {created_clips} video clips, skip {skipped_clips} existing"
        )
    else:
        logger.info(
            f"📊 Date {date}: Created {created_clips} video clips, skipped {skipped_clips} existing"
        )
    return stream_id


def main():
    """Main function to orchestrate the S3 import process."""
    parser = argparse.ArgumentParser(
        description="📥 Import S3 video objects into DynamoDB stream and video clip records for glowing-telegram."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="🔍 Don't actually create records, just show what would be done",
    )
    parser.add_argument(
        "--prefix",
        type=str,
        help="🔎 Only process objects with this prefix (e.g., '2023-08')",
    )
    parser.add_argument(
        "--date",
        type=str,
        help="📅 Only process objects for this specific date (YYYY-MM-DD)",
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="🐛 Enable verbose logging"
    )
    parser.add_argument(
        "--bucket",
        type=str,
        default=DEFAULT_BUCKET_NAME,
        help=f"🪣 S3 bucket name (default: {DEFAULT_BUCKET_NAME})",
    )
    parser.add_argument(
        "--streams-table",
        type=str,
        default=DEFAULT_STREAMS_TABLE,
        help=f"📊 DynamoDB streams table name (default: {DEFAULT_STREAMS_TABLE})",
    )
    parser.add_argument(
        "--video-metadata-table",
        type=str,
        default=DEFAULT_VIDEO_METADATA_TABLE,
        help=f"🎬 DynamoDB video metadata table name (default: {DEFAULT_VIDEO_METADATA_TABLE})",
    )

    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    if args.dry_run:
        logger.info("🔍 Running in DRY RUN mode - no records will be created")

    logger.info("🚀 Starting S3 import script for glowing-telegram")

    try:
        # Determine prefix to use
        prefix = args.prefix or ""
        if args.date:
            prefix = args.date
            logger.info(f"📅 Processing only objects for date: {args.date}")
        elif args.prefix:
            logger.info(f"🔍 Processing only objects with prefix: {args.prefix}")

        # Initialize AWS clients
        try:
            s3_client: S3Client = boto3.client("s3")  # type: ignore
            dynamodb: DynamoDBServiceResource = boto3.resource("dynamodb")  # type: ignore
        except Exception as e:
            logger.error(
                f"❌ Failed to initialize AWS clients. Please ensure AWS credentials are configured."
            )
            logger.error(f"Error details: {e}")
            raise S3ImportError("AWS credentials not configured or invalid")

        # List all video objects in the bucket
        try:
            objects = list_video_objects(s3_client, args.bucket, prefix)
        except Exception as e:
            if "NoSuchBucket" in str(e):
                logger.error(
                    f"❌ S3 bucket '{args.bucket}' does not exist or is not accessible"
                )
            else:
                logger.error(f"❌ Failed to list objects from S3: {e}")
            raise S3ImportError("Failed to access S3 bucket")

        if not objects:
            logger.warning("⚠️ No video objects found matching the expected pattern")
            return

        # Filter by specific date if requested
        if args.date:
            objects = [obj for obj in objects if obj["date"] == args.date]
            if not objects:
                logger.warning(f"⚠️ No objects found for date {args.date}")
                return

        # Group objects by date
        date_groups = group_objects_by_date(objects)

        # Process each date group
        total_streams_processed = 0
        total_clips_processed = 0

        for date, date_objects in sorted(date_groups.items()):
            stream_id = process_date_group(
                dynamodb.Table(args.streams_table),
                dynamodb.Table(args.video_metadata_table),
                date,
                date_objects,
                args.dry_run,
            )
            if stream_id:
                total_streams_processed += 1
                # Count clips that would be processed
                for obj in date_objects:
                    if not video_clip_exists(
                        dynamodb.Table(args.video_metadata_table), obj["key"]
                    ):
                        total_clips_processed += 1

        if args.dry_run:
            logger.info(f"🔍 [DRY RUN] Import simulation completed!")
            logger.info(
                f"📈 Summary: Would process {len(date_groups)} dates, "
                f"affect {total_streams_processed} streams, "
                f"create {total_clips_processed} video clips"
            )
        else:
            logger.info(f"🎉 Import completed successfully!")
            logger.info(
                f"📈 Summary: Processed {len(date_groups)} dates, "
                f"created/updated {total_streams_processed} streams, "
                f"created {total_clips_processed} video clips"
            )

    except Exception as e:
        logger.error(f"❌ Import failed: {e}")
        raise


if __name__ == "__main__":
    main()
