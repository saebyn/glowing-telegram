"""
Migrate our postgresql database to dynamodb

This script will be used to migrate our postgresql database to dynamodb.

tables to migrate:

- streams (streams-963700c)
  - id
  - title
  - description
  - prefix
  - thumbnail_url
  - created_at
  - updated_at
  - stream_id
  - stream_platform
  - duration
  - stream_date
  - series_id
- episodes (episodes-03b1f6f)
  - id
  - title
  - description
  - thumbnail_url
  - created_at
  - updated_at
  - stream_id
  - series_id
  - order_index
  - render_uri
  - is_published
  - notify_subscribers
  - category
  - tags
  - youtube_video_id
- video_clips (metadata-table-aa16405)
  - filename
  - stream_id
  - start_time
"""

from decimal import Decimal
import psycopg2
import boto3

streams = []
video_clips = []
episodes = []

# Connect to the postgresql database
with psycopg2.connect(
    host="localhost",
    database="video_processing_project",
    user="postgres",
    password="postgres",
) as conn:
    # Fetch the streams from the postgresql database
    with conn.cursor() as cur:
        cur.execute(
            """
            SELECT
                id,
                title,
                description,
                prefix,
                thumbnail_url,
                created_at,
                updated_at,
                stream_id,
                stream_platform,
                duration,
                stream_date,
                series_id
            FROM
                streams
        """
        )
        for row in cur.fetchall():
            streams.append(
                {
                    "id": row[0],
                    "title": row[1],
                    "description": row[2],
                    "prefix": row[3],
                    "thumbnail_url": row[4],
                    "created_at": row[5],
                    "updated_at": row[6],
                    "stream_id": row[7],
                    "stream_platform": row[8],
                    "duration": row[9],
                    "stream_date": row[10],
                    "series_id": row[11],
                }
            )

    # Fetch the video_clips from the postgresql database
    with conn.cursor() as cur:
        cur.execute(
            """
            SELECT
                start_time,
                stream_id,
                filename
            FROM
                video_clips
        """
        )

        for row in cur.fetchall():
            video_clips.append(
                {
                    "start_time": row[0],
                    "stream_id": row[1],
                    "filename": row[2],
                }
            )

    # Fetch the episodes from the postgresql database
    with conn.cursor() as cur:
        cur.execute(
            """
            SELECT
                id,
                title,
                description,
                thumbnail_url,
                created_at,
                updated_at,
                stream_id,
                series_id,
                order_index,
                render_uri,
                is_published,
                notify_subscribers,
                category,
                tags,
                youtube_video_id
            FROM
                episodes
        """
        )

        for row in cur.fetchall():
            episodes.append(
                {
                    "id": row[0],
                    "title": row[1],
                    "description": row[2],
                    "thumbnail_url": row[3],
                    "created_at": row[4],
                    "updated_at": row[5],
                    "stream_id": row[6],
                    "series_id": row[7],
                    "order_index": row[8],
                    "render_uri": row[9],
                    "is_published": row[10],
                    "notify_subscribers": row[11],
                    "category": row[12],
                    "tags": row[13],
                    "youtube_video_id": row[14],
                }
            )

# Connect to the dynamodb database
dynamodb = boto3.resource("dynamodb")

streams_table = dynamodb.Table("streams-963700c")

# Insert the streams into the dynamodb table
# overwriting the existing data!!
with streams_table.batch_writer() as batch:
    for stream in streams:
        stream_date = (
            stream["stream_date"].isoformat() if stream["stream_date"] else None
        )
        duration = (
            Decimal(stream["duration"].total_seconds()) if stream["duration"] else None
        )

        created_at = stream["created_at"].isoformat() if stream["created_at"] else None
        updated_at = stream["updated_at"].isoformat() if stream["updated_at"] else None

        batch.put_item(
            Item={
                "id": stream["id"],
                "title": stream["title"],
                "description": stream["description"],
                "prefix": stream["prefix"],
                "thumbnail_url": stream["thumbnail_url"],
                "created_at": created_at,
                "updated_at": updated_at,
                "stream_id": stream["stream_id"],
                "stream_platform": stream["stream_platform"] or "twitch",
                "duration": duration,
                "stream_date": stream_date,
                "series_id": stream["series_id"],
            }
        )

# episodes
episodes_table = dynamodb.Table("episodes-03b1f6f")
# overwriting the existing data!!
with episodes_table.batch_writer() as batch:
    for episode in episodes:
        created_at = (
            episode["created_at"].isoformat() if episode["created_at"] else None
        )
        updated_at = (
            episode["updated_at"].isoformat() if episode["updated_at"] else None
        )

        batch.put_item(
            Item={
                "id": episode["id"],
                "title": episode["title"],
                "description": episode["description"],
                "thumbnail_url": episode["thumbnail_url"],
                "created_at": created_at,
                "updated_at": updated_at,
                "stream_id": episode["stream_id"],
                "series_id": episode["series_id"],
                "order_index": episode["order_index"],
                "render_uri": episode["render_uri"],
                "is_published": episode["is_published"],
                "notify_subscribers": episode["notify_subscribers"],
                "category": episode["category"],
                "tags": episode["tags"],
                "youtube_video_id": episode["youtube_video_id"],
            }
        )


# sync the video_clips table to dynamodb
# ensure that we do not overwrite the existing data (so we cannot use batch_writer or put_item)
video_metadata_table = dynamodb.Table("metadata-table-aa16405")

for video_clip in video_clips:
    video_date = video_clip["filename"].split(" ")[0]
    key = f"{video_date}/{video_clip['filename']}"

    start_time = (
        Decimal(video_clip["start_time"].total_seconds())
        if video_clip["start_time"] is not None
        else None
    )

    video_metadata_table.update_item(
        Key={
            "key": key,
        },
        UpdateExpression="SET stream_id = :stream_id, start_time = :start_time",
        ExpressionAttributeValues={
            ":stream_id": video_clip["stream_id"],
            ":start_time": start_time,
        },
    )
