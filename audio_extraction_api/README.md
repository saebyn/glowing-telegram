# audio_extraction_api

This is a microservice that extracts audio from a set of video files. It is written in Rust. It is a port of the Python implementation.

Note for future self:
See assemble_audio_from_clips function in ../video_processing_project/video_app/media.py for a contrasting example in Python. This is a Rust implementation of that function. One significant difference is that this Rust API service needs to process the video files in segments to facilitate parallel and/or asynchronous processing. The Python implementation processes the video files in a single pass.

## API

This is a microservice that extracts one audio track from a set of video files. It provides four endpoints: `/extract_audio`, `/extract_audio_segment`, `/status`, and `/health`.

### `/extract_audio_segment`

The `/extract_audio_segment` API endpoint incrementally extracts audio from a set of video files. The API endpoint takes a list of video files, a track, and an optional cursor. The cursor is used to track the progress of the extraction. The API endpoint returns a new cursor that can be used to track the progress of the extraction. The API endpoint also returns a URI to the extracted audio file.

It accepts a POST request with a JSON body. The JSON body has three fields: `video_files`, `track`, and `cursor`. `video_files` is a list of URIs to video files. `track` is an optional integer that specifies which audio track to extract. If `track` is not provided, the first audio track will be extracted. `cursor` is an optional string that is used to track the progress of the extraction. If `cursor` is not provided, the extraction will start from the beginning. If `cursor` is provided, the extraction will start from the point where it left off.

The API endpoint returns a JSON response with two fields: `cursor` and `audio_file`. `cursor` is a string that can be used to track the progress of the extraction. `audio_file` is a URI to the extracted audio file.

The API endpoint is idempotent. If the API endpoint is called with the same `video_files` and `cursor` multiple times, it will return the same `cursor` and `audio_file` each time.

### `/extract_audio`

The `/extract_audio` endpoint is a convenience endpoint that accepts a POST request with a JSON body. The JSON body has two fields: `video_files` and `track`. `video_files` is a list of URIs to video files. `track` is an optional integer that specifies which audio track to extract. If `track` is not provided, the first audio track will be extracted. The API endpoint is asynchronous. It returns a response immediately, but the extraction happens in the background. The API endpoint returns a `202 Accepted` response. The response body is empty. The `Location` header contains a URL to the status endpoint for the extraction. The status endpoint can be polled to check the status of the extraction.

### `/status`

The `/status` endpoint is used to check the status of an extraction. It accepts a GET request. The URL contains a cursor that is used to track the progress of the extraction. The endpoint returns a JSON response with two fields: `status` and `audio_file`. `status` is a string that indicates the status of the extraction. `audio_file` is a URI to the extracted audio file. If the extraction is not complete, `audio_file` will be `null`.

### `/health`

The `/health` endpoint is used to check the health of the microservice. It accepts a GET request. The endpoint returns a `200 OK` response. The response body is empty.
