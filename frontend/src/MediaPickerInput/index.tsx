import { FC } from "react";
import { InputProps, Loading, useDataProvider, useInput } from "react-admin";
import { useQuery } from "@tanstack/react-query";
import MediaPicker, { MediaEntry } from "./MediaPicker";

const MediaPickerInput: FC<InputProps> = (props) => {
  const {
    field: { value, onChange },
  } = useInput(props);

  const dataProvider = useDataProvider();
  const { data, isLoading, error } = useQuery({
    queryKey: ["getRenderedEpisodeFiles"],
    queryFn: () => dataProvider.getRenderedEpisodeFiles(),
  });

  if (isLoading) return <Loading />;
  if (error)
    return (
      <div>
        <p>There was an error loading the media files</p>
        <pre>{JSON.stringify(error, null, 2)}</pre>
      </div>
    );
  if (!data) return null;

  const entries: MediaEntry[] = data.entries;

  return (
    <MediaPicker
      label={props.label}
      value={value}
      onChoose={(entry) => onChange(entry.uri)}
      entries={entries}
    />
  );
};

export default MediaPickerInput;
