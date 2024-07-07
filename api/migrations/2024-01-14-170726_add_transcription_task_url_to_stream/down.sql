-- This file should undo anything in `up.sql`
ALTER TABLE streams DROP COLUMN transcription_task_url;
ALTER TABLE streams DROP COLUMN transcription_segments;