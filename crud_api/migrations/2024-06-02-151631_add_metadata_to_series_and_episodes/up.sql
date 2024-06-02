-- Your SQL goes here
alter table series
  add column "notify_subscribers" boolean not null default false,
  add column "category" smallint not null default 20,
  add column "tags" text[] not null default '{}';

alter table episodes
  add column "notify_subscribers" boolean not null default false,
  add column "category" smallint not null default 20,
  add column "tags" text[] not null default '{}';