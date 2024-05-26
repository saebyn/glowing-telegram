-- This file should undo anything in `up.sql`
alter table episodes
  drop column "series_id",
  drop column "order_index";
