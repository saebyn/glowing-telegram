import {
  useRecordContext,
  Button,
  useDataProvider,
  ArrayInput,
  DateTimeInput,
  SimpleFormIterator,
  TextInput,
} from "react-admin";
import { useFormContext } from "react-hook-form";
import { useMutation } from "react-query";
import TitleInput from "../../TitleInput";
import {
  validateAudioBitrate,
  validateAudioTrackCount,
  validateContentType,
  validateFilename,
  validateFrameRate,
  validateHeight,
  validateWidth,
  validateVideoBitrate,
  validateSize,
  validateUri,
} from "../../validators";

interface Clip {
  uri: string;
  metadata: {
    title: string;
    duration: number;
    start_time: number;
    audio_bitrate: number;
    audio_track_count: number;
    content_type: string;
    filename: string;
    frame_rate: number;
    height: number;
    width: number;
    video_bitrate: number;
    size: number;
    last_modified: string;
  };
}

interface FindClipsResponse {
  entries: Clip[];
}

interface Entry {
  uri: string;
  title: string;
  duration: number;
  start_time: number;
  audio_bitrate: number;
  audio_track_count: number;
  content_type: string;
  filename: string;
  frame_rate: number;
  height: number;
  width: number;
  video_bitrate: number;
  size: number;
  last_modified: string;
}

const ScanButton = (props: any) => {
  const formContext = useFormContext();
  const record = useRecordContext();

  const dataProvider = useDataProvider();

  const { mutate, isLoading } = useMutation<FindClipsResponse>(() =>
    dataProvider.getStreamClips(record.prefix)
  );

  const prefix = record && record.prefix;
  const scanForClips = () => {
    mutate(void 0, {
      onSuccess: (data) => {
        const values: Entry[] = data.entries.map((entry: Clip) => ({
          title: entry.metadata.title || entry.metadata.filename,
          uri: entry.uri,
          duration: entry.metadata.duration,
          start_time: entry.metadata.start_time,
          audio_bitrate: entry.metadata.audio_bitrate,
          audio_track_count: entry.metadata.audio_track_count,
          content_type: entry.metadata.content_type,
          filename: entry.metadata.filename,
          frame_rate: entry.metadata.frame_rate,
          height: entry.metadata.height,
          width: entry.metadata.width,
          video_bitrate: entry.metadata.video_bitrate,
          size: entry.metadata.size,
          last_modified: entry.metadata.last_modified,
        }));

        formContext.setValue(props.source, values, {
          shouldValidate: true,
          shouldDirty: true,
        });
      },
    });
  };

  return (
    <Button
      disabled={!prefix || isLoading}
      label="Scan for Clips"
      onClick={scanForClips}
    />
  );
};

const FormIterator = ({ children, ...props }: any) => {
  return (
    <>
      <ScanButton source={props.source} />

      <SimpleFormIterator {...props}>{children}</SimpleFormIterator>
    </>
  );
};

const StreamVideoClipsInput = (props: any) => {
  return (
    <ArrayInput {...props}>
      <FormIterator inline>
        <TitleInput source="title" />
        <TextInput source="uri" validate={validateUri} />
        <TextInput source="duration" />
        <TextInput source="start_time" />
        <TextInput source="audio_bitrate" validate={validateAudioBitrate} />
        <TextInput
          source="audio_track_count"
          validate={validateAudioTrackCount}
        />
        <TextInput source="content_type" validate={validateContentType} />
        <TextInput source="filename" validate={validateFilename} />
        <TextInput source="frame_rate" validate={validateFrameRate} />
        <TextInput source="height" validate={validateHeight} />
        <TextInput source="width" validate={validateWidth} />
        <TextInput source="video_bitrate" validate={validateVideoBitrate} />
        <TextInput source="size" validate={validateSize} />
        <DateTimeInput source="last_modified" />
      </FormIterator>
    </ArrayInput>
  );
};

export default StreamVideoClipsInput;
