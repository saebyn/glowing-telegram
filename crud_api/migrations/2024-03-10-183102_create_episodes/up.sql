-- Your SQL goes here
create table topics
(
    id uuid not null default gen_random_uuid(),
    title character varying not null,
    description text not null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone,
    primary key (id)
);

-- Create trigger to update updated_at column
create trigger update_topics_updated_at before update on topics for each row execute procedure diesel_set_updated_at();

create table series
(
    id uuid not null default gen_random_uuid(),
    title character varying not null,
    description text not null,
    thumbnail_url character varying null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone,
    primary key (id)
);

-- Create trigger to update updated_at column
create trigger update_series_updated_at before update on series for each row execute procedure diesel_set_updated_at();

create table topic_series
(
    id uuid not null default gen_random_uuid(),
    topic_id uuid not null,
    series_id uuid not null,
    primary key (id),
    foreign key (topic_id) references topics (id),
    foreign key (series_id) references series (id)
);

create table episodes
(
    id uuid not null default gen_random_uuid(),
    title character varying not null,
    description text not null,
    thumbnail_url character varying null,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone,
    stream_id uuid not null,
    tracks jsonb not null,
    primary key (id),
    foreign key (stream_id) references streams (id)
);

-- Create trigger to update updated_at column
create trigger update_episodes_updated_at before update on episodes for each row execute procedure diesel_set_updated_at();

create table topic_episodes
(
    id uuid not null default gen_random_uuid(),
    topic_id uuid not null,
    episode_id uuid not null,
    primary key (id),
    foreign key (topic_id) references topics (id),
    foreign key (episode_id) references episodes (id)
);