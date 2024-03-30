-- This file should undo anything in `up.sql`
ALTER TABLE streams DROP COLUMN stream_id;
ALTER TABLE streams DROP COLUMN stream_platform;
ALTER TABLE streams DROP COLUMN duration;
ALTER TABLE streams DROP COLUMN stream_date;