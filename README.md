
# glowing-telegram


A tool for managing stream recordings.

This is a tool for managing stream recordings, ingesting them into a database, providing a web interface for searching, analyzing, and passing them to a video processing pipeline.

[I'm developing this tool live on Twitch. Why not come check it out sometime?](https://twitch.tv/saebyn) I'm developing this tool to practice my Rust, as it's a bit rusty, and to automate some of the video processing tasks that I do manually by spending way more time programming than I would have spent doing the tasks manually.

## Features

1. Track locally recorded clips from a stream
1. Generate a set of "episodes" from the stream based on when the speaker is speaking
1. Episode transcription
1. Review interface for the transcriptions
1. Automatic summaries of the episode via text summarization provided by GPT-4
1. Flag areas of the video that are interesting
1. Generate a set of "highlights" from the stream based on the flagged areas
1. Archive the stream videos to a cloud storage provider

## Architecture

The tool is composed of a web interface, a database, and a set of microservices. The web interface is a React app that communicates with the microservices via a REST API. The microservices are written in a combination of Rust and Python. The database is a PostgreSQL database. The microservices do not share state, and most of them are stateless. Metadata about the stream is stored in the database, and the microservices use that metadata to perform their tasks. Video and audio data is stored in mounted volumes, and the microservices use the metadata to locate the data.

