import {
  ArrayInput,
  DateInput,
  DateTimeInput,
  Edit,
  EditProps,
  ReferenceArrayInput,
  SimpleForm,
  SimpleFormIterator,
  TextInput,
} from "react-admin";

const StreamEdit = (props: EditProps) => (
  <Edit {...props}>
    <SimpleForm>
      <TextInput source="title" />
      <TextInput multiline={true} source="description" />
      <DateInput source="prefix" />
      <TextInput source="speech_audio_track" />
      <TextInput source="thumbnail" />
      <ReferenceArrayInput source="topic_ids" reference="topics">
        <TextInput source="id" />
      </ReferenceArrayInput>

      <ArrayInput source="video_clips">
        <SimpleFormIterator inline>
          <TextInput source="title" required />
          <TextInput source="uri" />
          <TextInput source="duration" />
          <TextInput source="start_time" />
          <TextInput source="audio_bitrate" />
          <TextInput source="audio_track_count" />
          <TextInput source="content_type" />
          <TextInput source="filename" />
          <TextInput source="frame_rate" />
          <TextInput source="height" />
          <TextInput source="width" />
          <TextInput source="video_bitrate" />
          <TextInput source="size" />
          <DateTimeInput source="last_modified" />
        </SimpleFormIterator>
      </ArrayInput>
    </SimpleForm>
  </Edit>
);

export default StreamEdit;
