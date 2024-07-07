-- Your SQL goes here
-- add series_id and order to episodes
alter table episodes
  add column "series_id" uuid null,
  add column "order_index" integer not null default 0;

-- Add foreign key to series
alter table episodes
  add constraint "fk_series_id"
  foreign key ("series_id")
  references series (id);