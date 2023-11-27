# silence_detection_api

This is a microservice that detects silence in a set of audio files. It is written in Rust. It is a port of the Python implementation.

Note for future self:

See identify_episodes function in ../video_processing_project/video_app/media.py for a contrasting example in Python. This is a Rust implementation of that function. One significant difference is that this Rust API service needs to process the audio file in segments to facilitate parallel and/or asynchronous processing. The Python implementation processes the audio file in a single pass.

## API

This is a microservice that detects silence in a set of audio files. It provides three endpoints: `/detect_silence`, `/detect_silence_segment`, and `/health`.

### `/detect_silence_segment`

The `/detect_silence_segment` API endpoint incrementally detects silence in an audio file. The API endpoint takes an audio file URI and an optional cursor. The cursor is used to track the progress of the detection. The API endpoint returns a new cursor that can be used to track the progress of the detection. The API endpoint also returns a list of silence segments. A silence segment is a segment of the audio file that contains silence relative to the rest of the audio file.

It accepts a POST request with a JSON body. The JSON body has two fields: `audio_file` and `cursor`. `audio_file` is a URI to an audio file. `cursor` is an optional string that is used to track the progress of the detection. If `cursor` is not provided, the detection will start from the beginning. If `cursor` is provided, the detection will start from the point where it left off.

The API endpoint returns a JSON response with two fields: `cursor` and `silence_segments`. `cursor` is a string that can be used to track the progress of the detection. `silence_segments` is a list of silence segments. A silence segment is an object that has two fields: `start` and `end`. `start` is a duration until the start of the silence segment from the beginning of the audio file, as an ISO 8601 duration. `end` is a duration until the end of the silence segment from the beginning of the audio file, as an ISO 8601 duration.

The API endpoint is idempotent. If the API endpoint is called with the same `audio_file` and `cursor` multiple times, it will return the same `cursor` and `silence_segments` each time.

### `/detect_silence`

The `/detect_silence` endpoint is a convenience endpoint that accepts a POST request with a JSON body. The JSON body has one field: `audio_file`. `audio_file` is a URI to an audio file. The API endpoint is asynchronous. It returns a response immediately, but the detection happens in the background. The API endpoint returns a `202 Accepted` response. The response body is empty. The `Location` header contains a URL to the status endpoint for the detection. The status endpoint can be polled to check the status of the detection.

### `/status`

The `/status` endpoint is used to check the status of a detection. It accepts a GET request. The URL contains a cursor that is used to track the progress of the detection. The endpoint returns a JSON response with two fields: `status` and `silence_segments`. `status` is a string that indicates the status of the detection. `silence_segments` is a list of silence segments. A silence segment is an object that has two fields: `start` and `end`. `start` is a duration until the start of the silence segment from the beginning of the audio file, as an ISO 8601 duration. `end` is a duration until the end of the silence segment from the beginning of the audio file, as an ISO 8601 duration. If the detection is not complete, `silence_segments` will contain the silence segments that have been detected so far.

### `/health`

The `/health` endpoint is used to check the health of the microservice. It accepts a GET request. The endpoint returns a `200 OK` response. The response body is empty.
