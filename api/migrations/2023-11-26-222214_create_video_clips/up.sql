-- Your SQL goes here
CREATE TABLE video_clips (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    title character varying NOT NULL,
    description text NOT NULL,
    url character varying NOT NULL,
    duration interval NOT NULL,
    start_time interval NOT NULL,

    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone,

    stream_id uuid NULL REFERENCES streams(id) ON DELETE CASCADE ON UPDATE CASCADE,

    PRIMARY KEY (id)
);

-- Create trigger to update updated_at column
CREATE TRIGGER update_video_clips_updated_at BEFORE UPDATE ON video_clips FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at();