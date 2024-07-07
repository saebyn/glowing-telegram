-- Your SQL goes here
ALTER TABLE video_clips
ADD COLUMN audio_bitrate INTEGER,
ADD COLUMN audio_track_count INTEGER,
ADD COLUMN content_type VARCHAR(255),
ADD COLUMN filename VARCHAR(255),
ADD COLUMN frame_rate REAL,
ADD COLUMN height INTEGER,
ADD COLUMN width INTEGER,
ADD COLUMN video_bitrate INTEGER,
ADD COLUMN size BIGINT,
ADD COLUMN last_modified TIMESTAMP;

ALTER TABLE video_clips
RENAME COLUMN url TO uri;
