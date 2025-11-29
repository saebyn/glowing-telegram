import boto3
import os
import re

VIDEO_METADATA_TABLE = os.environ["VIDEO_METADATA_TABLE"]
STREAM_ID_INDEX = os.environ["STREAM_ID_INDEX"]

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
    for item in response.get('Items', []):
        yield item
    
    while 'LastEvaluatedKey' in response:
        kwargs['ExclusiveStartKey'] = response['LastEvaluatedKey']
        response = table.query(**kwargs)
        for item in response.get('Items', []):
            yield item


def handler(event, context):
    """ """
    path = event["rawPath"]
    stream_id = re.match(r"/playlist/([^/]+).m3u8", path).group(1)

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
        },
        "body": m3u8_playlist_text,
    }


import urllib.parse


def rewrite_path(path):
    path = path.replace("transcode/", "/")
    # make path url safe
    return urllib.parse.quote(path)
