-- Your SQL goes here
CREATE TABLE streams
(
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    title character varying NOT NULL,
    description text NOT NULL,
    prefix character varying NOT NULL,
    speech_audio_url character varying NOT NULL,
    thumbnail_url character varying NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone,
    PRIMARY KEY (id)
);

-- Create trigger to update updated_at column
CREATE TRIGGER update_streams_updated_at BEFORE UPDATE ON streams FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at();