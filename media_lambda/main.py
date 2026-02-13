import boto3
import os
import re

VIDEO_METADATA_TABLE = os.environ["VIDEO_METADATA_TABLE"]
STREAM_ID_INDEX = os.environ["STREAM_ID_INDEX"]
PROJECTS_TABLE = os.environ.get("PROJECTS_TABLE", "")
DEFAULT_FPS = float(os.environ.get("DEFAULT_FPS", "60"))

M3U8_HEADER = """#EXTM3U
#EXT-X-VERSION:3
#EXT-X-PLAYLIST-TYPE:VOD
#EXT-X-TARGETDURATION:4
#EXT-X-MEDIA-SEQUENCE:0"""

M3U8_SEGMENT = """#EXTINF:{duration}\n{path}"""

M3U8_FOOTER = """#EXT-X-ENDLIST\n"""

# Initialize DynamoDB resource
dynamodb = boto3.resource("dynamodb")


def paginated_query(table, **kwargs):
    """Generator that yields items from a DynamoDB query, handling pagination automatically."""
    response = table.query(**kwargs)
    for item in response.get("Items", []):
        yield item

    while "LastEvaluatedKey" in response:
        kwargs["ExclusiveStartKey"] = response["LastEvaluatedKey"]
        response = table.query(**kwargs)
        for item in response.get("Items", []):
            yield item


def handler(event, context):
    """Main handler that routes to either stream or project playlist generation."""
    path = event["rawPath"]
    
    # Check if this is a project playlist request
    project_match = re.match(r"/playlist/project/([^/]+)\.m3u8", path)
    if project_match:
        return handle_project_playlist(project_match.group(1))
    
    # Otherwise handle as stream playlist
    stream_match = re.match(r"/playlist/([^/]+)\.m3u8", path)
    if stream_match:
        return handle_stream_playlist(stream_match.group(1))
    
    return {
        "statusCode": 400,
        "headers": {
            "Content-Type": "text/plain",
            "Cache-Control": "no-cache, no-store, must-revalidate",
            "Pragma": "no-cache",
            "Expires": "0",
        },
        "body": "Invalid playlist path",
    }


def handle_stream_playlist(stream_id):
    """Generate playlist for a stream (all videos in order)."""
    if not stream_id:
        return {
            "statusCode": 400,
            "body": "Invalid stream ID",
        }

    table = dynamodb.Table(VIDEO_METADATA_TABLE)

    stream_video_records = [
        {"key": item["key"], "transcode": item["transcode"]}
        for item in paginated_query(
            table,
            IndexName=STREAM_ID_INDEX,
            KeyConditionExpression="stream_id = :streamId",
            ExpressionAttributeValues={":streamId": stream_id},
            ProjectionExpression="#key, transcode",
            ExpressionAttributeNames={"#key": "key"},
        )
        if "transcode" in item
    ]

    print(f"Found {len(stream_video_records)} video records")

    sorted_stream_videos = sorted(stream_video_records, key=lambda x: x["key"])

    lines = []

    for video_record in sorted_stream_videos:
        lines.append(f"#EXT-X-DISCONTINUITY")
        for segment in video_record["transcode"]:
            lines.append(f"#EXTINF:{segment['duration']}")
            path = rewrite_path(segment["path"])
            lines.append(path)

    print(f"Found {len(lines)} segments")

    # Create the m3u8 playlist text
    m3u8_playlist_text = "\n".join(
        [
            M3U8_HEADER,
            *lines,
            M3U8_FOOTER,
        ]
    )

    print(f"length of m3u8_playlist_text: {len(m3u8_playlist_text)}")

    return {
        "statusCode": 200,
        "headers": {
            "Content-Type": "audio/mpegurl",
            "Cache-Control": "no-cache, no-store, must-revalidate",
            "Pragma": "no-cache",
            "Expires": "0",
        },
        "body": m3u8_playlist_text,
    }


def handle_project_playlist(project_id):
    """Generate playlist for a project based on its cut_list or video_clip_ids."""
    if not project_id:
        return {
            "statusCode": 400,
            "body": "Invalid project ID",
        }

    if not PROJECTS_TABLE:
        return {
            "statusCode": 500,
            "body": "Projects table not configured",
        }

    # Fetch project from DynamoDB
    projects_table = dynamodb.Table(PROJECTS_TABLE)
    try:
        response = projects_table.get_item(Key={"id": project_id})
    except Exception as e:
        print(f"Error fetching project: {e}")
        return {
            "statusCode": 500,
            "headers": {
                "Content-Type": "text/plain",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache",
                "Expires": "0",
            },
            "body": "Internal server error while fetching project",
        }

    if "Item" not in response:
        return {
            "statusCode": 404,
            "headers": {
                "Content-Type": "text/plain",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache",
                "Expires": "0",
            },
            "body": "Project not found",
        }

    project = response["Item"]
    print(f"Found project: {project.get('title', project_id)}")

    # Try to use cut_list first, then fall back to video_clip_ids
    if "cut_list" in project and project["cut_list"]:
        return handle_project_with_cut_list(project)
    elif "video_clip_ids" in project and project["video_clip_ids"]:
        return handle_project_with_clip_ids(project)
    else:
        return {
            "statusCode": 400,
            "headers": {
                "Content-Type": "text/plain",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache",
                "Expires": "0",
            },
            "body": "Project has no cut_list or video_clip_ids defined",
        }


def handle_project_with_cut_list(project):
    """
    Generate playlist using project's cut_list.
    
    Note: Uses configured DEFAULT_FPS for frame-to-time conversion.
    In a full implementation, frame rate should be extracted from video metadata.
    """
    cut_list = project["cut_list"]
    
    if "inputMedia" not in cut_list or "outputTrack" not in cut_list:
        return {
            "statusCode": 400,
            "headers": {
                "Content-Type": "text/plain",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache",
                "Expires": "0",
            },
            "body": "Invalid cut_list structure",
        }

    input_media = cut_list["inputMedia"]
    output_track = cut_list["outputTrack"]

    print(f"Processing cut_list with {len(input_media)} input media and {len(output_track)} output tracks")

    # Get video metadata table
    video_table = dynamodb.Table(VIDEO_METADATA_TABLE)

    # Collect all segments from output track
    lines = []
    prev_media_index = None

    for track_item in output_track:
        # Convert from DynamoDB Decimal to int for indexing
        media_index = int(track_item["mediaIndex"])
        section_index = int(track_item["sectionIndex"])

        if media_index >= len(input_media):
            print(f"Warning: mediaIndex {media_index} out of range")
            continue

        media = input_media[media_index]
        s3_location = media["s3Location"]

        if section_index >= len(media.get("sections", [])):
            print(f"Warning: sectionIndex {section_index} out of range for media {media_index}")
            continue

        section = media["sections"][section_index]
        # Convert from DynamoDB Decimal to float for calculations
        start_frame = float(section["startFrame"])
        end_frame = float(section["endFrame"])

        # Fetch video clip data
        try:
            response = video_table.get_item(Key={"key": s3_location})
        except Exception as e:
            print(f"Error fetching video clip {s3_location}: {e}")
            continue

        if "Item" not in response:
            print(f"Warning: Video clip not found: {s3_location}")
            continue

        video_clip = response["Item"]

        if "transcode" not in video_clip or not video_clip["transcode"]:
            print(f"Warning: No transcode data for video clip: {s3_location}")
            continue

        # Get metadata to calculate frame rate
        metadata = video_clip.get("metadata", {})
        format_info = metadata.get("format", {})
        duration = format_info.get("duration", 0)

        if duration <= 0:
            print(f"Warning: Invalid duration for video clip: {s3_location}")
            continue

        # Use configured default frame rate if not specified in metadata
        # TODO: Extract actual frame rate from video metadata (e.g., from metadata.format.r_frame_rate)
        fps = DEFAULT_FPS

        # Convert frames to time
        start_time = start_frame / fps
        end_time = end_frame / fps

        # Filter segments that fall within the time range
        segments = get_segments_in_range(video_clip["transcode"], start_time, end_time)

        if not segments:
            print(f"Warning: No segments found in range {start_time}-{end_time} for {s3_location}")
            continue

        # Add discontinuity tag when switching between different media sources
        if prev_media_index is not None and prev_media_index != media_index:
            lines.append("#EXT-X-DISCONTINUITY")

        prev_media_index = media_index

        # Add segments to playlist
        for segment in segments:
            lines.append(f"#EXTINF:{segment['duration']}")
            path = rewrite_path(segment["path"])
            lines.append(path)

    if not lines:
        return {
            "statusCode": 400,
            "headers": {
                "Content-Type": "text/plain",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache",
                "Expires": "0",
            },
            "body": "No valid segments found in project cut_list",
        }

    print(f"Generated {len(lines)} playlist lines from cut_list")

    # Create the m3u8 playlist text
    m3u8_playlist_text = "\n".join(
        [
            M3U8_HEADER,
            *lines,
            M3U8_FOOTER,
        ]
    )

    return {
        "statusCode": 200,
        "headers": {
            "Content-Type": "audio/mpegurl",
            "Cache-Control": "no-cache, no-store, must-revalidate",
            "Pragma": "no-cache",
            "Expires": "0",
        },
        "body": m3u8_playlist_text,
    }


def handle_project_with_clip_ids(project):
    """Generate playlist using project's video_clip_ids."""
    video_clip_ids = project["video_clip_ids"]
    print(f"Processing project with {len(video_clip_ids)} video clip IDs")

    video_table = dynamodb.Table(VIDEO_METADATA_TABLE)
    lines = []
    prev_clip_key = None

    for clip_id in video_clip_ids:
        # Fetch video clip data
        try:
            response = video_table.get_item(Key={"key": clip_id})
        except Exception as e:
            print(f"Error fetching video clip {clip_id}: {e}")
            continue

        if "Item" not in response:
            print(f"Warning: Video clip not found: {clip_id}")
            continue

        video_clip = response["Item"]

        if "transcode" not in video_clip or not video_clip["transcode"]:
            print(f"Warning: No transcode data for video clip: {clip_id}")
            continue

        # Add discontinuity tag when switching between different clips
        if prev_clip_key is not None:
            lines.append("#EXT-X-DISCONTINUITY")

        prev_clip_key = clip_id

        # Add all segments from this video clip
        for segment in video_clip["transcode"]:
            lines.append(f"#EXTINF:{segment['duration']}")
            path = rewrite_path(segment["path"])
            lines.append(path)

    if not lines:
        return {
            "statusCode": 400,
            "headers": {
                "Content-Type": "text/plain",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache",
                "Expires": "0",
            },
            "body": "No valid segments found in project video_clip_ids",
        }

    print(f"Generated {len(lines)} playlist lines from video_clip_ids")

    # Create the m3u8 playlist text
    m3u8_playlist_text = "\n".join(
        [
            M3U8_HEADER,
            *lines,
            M3U8_FOOTER,
        ]
    )

    return {
        "statusCode": 200,
        "headers": {
            "Content-Type": "audio/mpegurl",
            "Cache-Control": "no-cache, no-store, must-revalidate",
            "Pragma": "no-cache",
            "Expires": "0",
        },
        "body": m3u8_playlist_text,
    }


def get_segments_in_range(transcode_segments, start_time, end_time):
    """
    Filter HLS segments that fall within the specified time range.
    
    Segments are included if they have any overlap with the desired time range.
    This means segments may extend slightly beyond the requested end_time, which
    is acceptable for HLS playback as players will handle the boundaries correctly.
    
    Args:
        transcode_segments: List of segment dicts with 'path' and 'duration'
        start_time: Start time in seconds
        end_time: End time in seconds
    
    Returns:
        List of segments that fall within the time range
    """
    result = []
    current_time = 0.0

    for segment in transcode_segments:
        # Convert from DynamoDB Decimal to float for calculations
        segment_duration = float(segment["duration"])
        segment_end = current_time + segment_duration

        # Include segment if it has any overlap with the desired range
        # Segment overlaps if: segment_end > start_time AND current_time < end_time
        if segment_end > start_time and current_time < end_time:
            result.append(segment)

        current_time = segment_end

        # Stop if we've passed the end time
        if current_time >= end_time:
            break

    return result


import urllib.parse


def rewrite_path(path):
    return urllib.parse.quote("/" + path)
