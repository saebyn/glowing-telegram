# stream_ingestion_api

## Description

This is a microservice that takes a prefix string and finds all of the files in storage that start with that prefix and returns a list of URIs to those files along with metadata about the files. Since the expected number of matching files is small, the microservice returns all of the matching files in a single response.

## API

### `/find_files`

The `/find_files` API endpoint takes a prefix string and returns a list of URIs to files that start with that prefix. It accepts a POST request with a JSON body. The JSON body has one field: `prefix`. `prefix` is a string that specifies the prefix to search for. The API endpoint returns a JSON response with one field: `files`. `files` is a list of objects. Each object has two fields: `urn` and `metadata`. `urn` is a string that contains a URI to a file. `metadata` is an object that contains metadata about the file. The API endpoint returns a `200 OK` response.

The `metadata` object contains the following fields:

- `content_type`: The content type of the file.
- `size`: The size of the file in bytes.
- `last_modified`: The last modified time of the file in ISO 8601 format.
- `md5`: The MD5 hash of the file.
- `duration` (optional): The duration of the file in seconds. This field is only present if the file is a video file.
- `width` (optional): The width of the file in pixels. This field is only present if the file is a video file.
- `height` (optional): The height of the file in pixels. This field is only present if the file is a video file.
- `frame_rate` (optional): The frame rate of the file in frames per second. This field is only present if the file is a video file.
- `bit_rate` (optional): The bit rate of the file in bits per second. This field is only present if the file is a video file.
- `audio_tracks` (optional): The number of audio tracks in the file. This field is only present if the file is a video file.

### `/health`

The `/health` endpoint is used to check the health of the microservice. It accepts a GET request. The endpoint returns a `200 OK` response. The response body is empty.
