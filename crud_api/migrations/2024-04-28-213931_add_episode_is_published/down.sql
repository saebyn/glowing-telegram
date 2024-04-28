-- This file should undo anything in `up.sql`
-- Remove the column is_published from the episodes table
ALTER TABLE episodes
DROP COLUMN is_published;