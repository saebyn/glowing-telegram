import {
  DateInput,
  Edit,
  EditProps,
  ReferenceArrayInput,
  SimpleForm,
  TextInput,
} from "react-admin";

const StreamEdit = (props: EditProps) => (
  <Edit {...props}>
    <SimpleForm>
      <TextInput source="title" />
      <TextInput source="description" />
      <DateInput source="prefix" />
      <TextInput source="speech_audio_track" />
      <TextInput source="thumbnail" />
      <ReferenceArrayInput source="topic_ids" reference="topics">
        <TextInput source="id" />
      </ReferenceArrayInput>
    </SimpleForm>
  </Edit>
);

export default StreamEdit;
