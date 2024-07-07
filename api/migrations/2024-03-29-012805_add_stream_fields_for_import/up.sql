-- Your SQL goes here
ALTER TABLE streams ADD COLUMN stream_id VARCHAR(255);
ALTER TABLE streams ADD COLUMN stream_platform VARCHAR(255);
ALTER TABLE streams ADD COLUMN duration interval NOT NULL DEFAULT '00:00:00';
ALTER TABLE streams ADD COLUMN stream_date timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP;