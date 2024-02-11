-- This file should undo anything in `up.sql`
ALTER TABLE streams DROP COLUMN silence_detection_task_url;
ALTER TABLE streams DROP COLUMN silence_segments;