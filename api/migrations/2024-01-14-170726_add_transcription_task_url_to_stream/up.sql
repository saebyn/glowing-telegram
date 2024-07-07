-- Your SQL goes here
ALTER TABLE streams ADD COLUMN transcription_task_url TEXT;
ALTER TABLE streams ADD COLUMN transcription_segments JSONB;
