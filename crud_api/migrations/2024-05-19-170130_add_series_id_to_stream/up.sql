
-- add series_id 
alter table streams
  add column "series_id" uuid null;

-- Add foreign key to series
alter table streams
  add constraint "fk_series_id"
  foreign key ("series_id")
  references series (id);