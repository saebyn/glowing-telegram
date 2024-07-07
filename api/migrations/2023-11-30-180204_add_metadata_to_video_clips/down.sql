-- This file should undo anything in `up.sql`
ALTER TABLE video_clips
DROP COLUMN audio_bitrate,
DROP COLUMN audio_track_count,
DROP COLUMN content_type,
DROP COLUMN filename,
DROP COLUMN frame_rate,
DROP COLUMN height,
DROP COLUMN width,
DROP COLUMN video_bitrate,
DROP COLUMN size,
DROP COLUMN last_modified;

ALTER TABLE video_clips
RENAME COLUMN uri TO url;
