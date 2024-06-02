-- This file should undo anything in `up.sql`
alter table series
  drop column "notify_subscribers",
  drop column "category",
  drop column "tags";

alter table episodes
  drop column "notify_subscribers",
  drop column "category",
  drop column "tags";