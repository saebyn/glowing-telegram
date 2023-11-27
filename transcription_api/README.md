# transcription_api

This is a microservice that transcribes an audio file. It is written in Rust. It is a port of the Python implementation.

Note for future self:

See transcribe_audio function in ../video_processing_project/transcription_app/transcription.py for a contrasting example in Python. This is a Rust implementation of that function. One significant difference is that this Rust API service needs to process the audio file in segments to facilitate parallel and/or asynchronous processing. The Python implementation processes the audio file in a single pass.

## API

This is a microservice that transcribes text from the selected track of an audio file. It provides four endpoints: `/transcribe_audio`, `/transcribe_audio_segment`, `/status`, and `/health`.

### `/transcribe_audio_segment`

The `/transcribe_audio_segment` API endpoint incrementally transcribes speech in an audio file. The API endpoint takes an audio file URI, an optional track number, and an optional cursor. The cursor is used to track the progress of the detection. The API endpoint returns a new cursor that can be used to request the transcription of the next segment. The API endpoint also returns the transcription of the segment.

It accepts a POST request with a JSON body. The JSON body has three fields: `audio_file`, `track`, and `cursor`. `audio_file` is a URI to an audio file. `track` is an optional integer that indicates which track of the audio file to transcribe. If `track` is not provided, the first track will be transcribed. `cursor` is an optional string that is used to track the progress of the transcription. If `cursor` is not provided, the transcription will start from the beginning. If `cursor` is provided, the transcription will start from the point where it left off.

The API endpoint returns a JSON response with two fields: `cursor` and `transcription`. `cursor` is a string that can be used to track the progress of the transcription. `transcription` is an object that has three fields: `text`, `start`, and `end`. `text` is the transcription of the segment. `start` is a duration until the start of the segment from the beginning of the audio file, as an ISO 8601 duration. `end` is a duration until the end of the segment from the beginning of the segment, as an ISO 8601 duration.

The API endpoint is intended to be idempotent. If the API endpoint is called with the same `audio_file`, `track`, and `cursor` multiple times, it will likely return the same `cursor` and `transcription` each time.

### `/transcribe_audio`

The `/transcribe_audio` endpoint is a convenience endpoint that accepts a POST request with a JSON body. The JSON body has two fields: `audio_file` and `track`. `audio_file` is a URI to an audio file. `track` is an optional integer that indicates which track of the audio file to transcribe. If `track` is not provided, the first track will be transcribed. The API endpoint is asynchronous. It returns a response immediately, but the transcription happens in the background. The API endpoint returns a `202 Accepted` response. The response body is empty. The `Location` header contains a URL to the status endpoint for the transcription. The status endpoint can be polled to check the status of the transcription.

### `/status`

The `/status` endpoint is used to check the status of a transcription. It accepts a GET request. The URL contains a cursor that is used to track the progress of the transcription. The endpoint returns a JSON response with two fields: `status` and `transcription`. `status` is a string that indicates the status of the transcription. `transcription` is a list of objects, each having three fields: `text`, `start`, and `end`. `text` is the transcription of the segment. `start` is a duration until the start of the segment from the beginning of the audio file, as an ISO 8601 duration. `end` is a duration until the end of the segment from the beginning of the segment, as an ISO 8601 duration. If the transcription is not complete, `transcription` will contain the segments that have been transcribed so far.

### `/health`

The `/health` endpoint is used to check the health of the microservice. It accepts a GET request. The endpoint returns a `200 OK` response. The response body is empty.
