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


def handler(event, context):
    """ """
    path = event["rawPath"]
    stream_id = re.match(r"/playlist/([^/]+).m3u8", path).group(1)

    if not stream_id:
        return {
            "statusCode": 400,
            "body": "Invalid stream ID",
        }

    # Use the DynamoDB client to get the video metadata
    dynamodb = boto3.client("dynamodb")

    # Query the table using the streamId index
    start_key = None

    stream_video_records = []

    while True:
        query_args = {
            "TableName": VIDEO_METADATA_TABLE,
            "IndexName": STREAM_ID_INDEX,
            "KeyConditionExpression": "stream_id = :streamId",
            "ExpressionAttributeValues": {":streamId": {"S": stream_id}},
            "ProjectionExpression": "#key, transcode",
            "ExpressionAttributeNames": {"#key": "key"},
        }

        if start_key:
            query_args["ExclusiveStartKey"] = start_key

        response = dynamodb.query(**query_args)

        stream_video_records.extend(
            [
                {"key": item["key"]["S"], "transcode": item["transcode"]["L"]}
                for item in response["Items"]
                if "transcode" in item
            ]
        )

        if "LastEvaluatedKey" in response:
            start_key = response["LastEvaluatedKey"]
        else:
            break

    sorted_stream_videos = sorted(stream_video_records, key=lambda x: x["key"])

    transcoded_video_segments = [
        {
            "path": segment["M"]["path"]["S"],
            "duration": segment["M"]["duration"]["N"],
        }
        for video_record in sorted_stream_videos
        for segment in video_record["transcode"]
    ]

    m3u8_playlist_text = "\n".join(
        [
            M3U8_HEADER,
            *[
                M3U8_SEGMENT.format(
                    path=rewrite_path(segment["path"]), duration=segment["duration"]
                )
                for segment in transcoded_video_segments
            ],
            M3U8_FOOTER,
        ]
    )

    return {
        "statusCode": 200,
        "headers": {
            "Content-Type": "audio/mpegurl",
        },
        "body": m3u8_playlist_text,
    }


def rewrite_path(path):
    return path.replace("transcode/", "/")
