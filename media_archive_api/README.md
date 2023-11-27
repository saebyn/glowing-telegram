# media_archive_api

This is a microservice that transfers media files from local storage to cloud storage.

## API

This is a microservice that transfers media files from local storage to cloud storage. It provides four endpoints: `/transfer_media`, `/transfer_media_segment`, `/status`, and `/health`.

### `/transfer_media_segment`

The `/transfer_media_segment` API endpoint incrementally transfers media files from local storage to cloud storage. The API endpoint takes a list of media files and an optional cursor. The cursor is used to track the progress of the transfer. The API endpoint returns a new cursor that can be used to track the progress of the transfer.

It accepts a POST request with a JSON body. The JSON body has two fields: `media_files` and `cursor`. `media_files` is a list of URIs to media files. `cursor` is an optional string that is used to track the progress of the transfer. If `cursor` is not provided, the transfer will start from the beginning. If `cursor` is provided, the transfer will start from the point where it left off.

The API endpoint returns a JSON response with one field: `cursor`. `cursor` is a string that can be used to track the progress of the transfer.

### `/transfer_media`

The `/transfer_media` endpoint is a convenience endpoint that accepts a POST request with a JSON body. The JSON body has one field: `media_files`. `media_files` is a list of URIs to media files. The API endpoint is asynchronous. It returns a response immediately, but the transfer happens in the background. The API endpoint returns a `202 Accepted` response. The response body is empty. The `Location` header contains a URL to the status endpoint for the transfer. The status endpoint can be polled to check the status of the transfer.

### `/status`

The `/status` endpoint is used to check the status of a transfer. It accepts a GET request. The URL contains a cursor that is used to track the progress of the transfer. The endpoint returns a JSON response with one field: `status`. `status` is a string that indicates the status of the transfer.

### `/health`

The `/health` endpoint is used to check the health of the microservice. It accepts a GET request. The endpoint returns a `200 OK` response. The response body is empty.
