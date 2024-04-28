-- Your SQL goes here
-- Add the column is_published to the episodes table
ALTER TABLE episodes
ADD COLUMN is_published boolean NOT NULL DEFAULT false;