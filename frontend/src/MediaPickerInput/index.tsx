import { FC } from "react";
import { Loading, useDataProvider, useInput } from "react-admin";
import { useQuery } from "react-query";
import MediaPicker, { MediaEntry } from "./MediaPicker";

const MediaPickerInput: FC<any> = (props) => {
  const {
    field: { value, onChange },
  } = useInput(props);

  const dataProvider = useDataProvider();
  const { data, isLoading, error } = useQuery(["getRenderedEpisodeFiles"], () =>
    dataProvider.getRenderedEpisodeFiles()
  );

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
