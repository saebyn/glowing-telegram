import {
  Edit,
  EditProps,
  ReferenceArrayInput,
  SimpleForm,
  TextInput,
} from "react-admin";

import StreamVideoClipsInput from "./StreamVideoClipsInput";

const StreamEdit = (props: EditProps) => (
  <Edit {...props}>
    <SimpleForm>
      <TextInput source="title" />
      <TextInput multiline={true} source="description" />
      <TextInput source="prefix" />
      <TextInput source="speech_audio_track" />
      <TextInput source="thumbnail" />
      <ReferenceArrayInput source="topic_ids" reference="topics">
        <TextInput source="id" />
      </ReferenceArrayInput>

      <StreamVideoClipsInput source="video_clips" />
    </SimpleForm>
  </Edit>
);

export default StreamEdit;
