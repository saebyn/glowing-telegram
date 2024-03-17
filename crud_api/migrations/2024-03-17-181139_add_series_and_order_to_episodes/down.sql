-- This file should undo anything in `up.sql`
alter table episodes
  drop column "series_id",
  drop column "order_index";

-- Remove foreign key to series
alter table episodes
  drop constraint "fk_series_id";
