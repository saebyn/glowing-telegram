-- Your SQL goes here
ALTER TABLE streams ADD COLUMN silence_detection_task_url TEXT;
ALTER TABLE streams ADD COLUMN silence_segments JSONB;