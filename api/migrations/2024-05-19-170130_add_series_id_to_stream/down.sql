-- This file should undo anything in `up.sql`
alter table streams
  drop "series_id";
