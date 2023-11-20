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
      <DateInput source="created_at" />
      <TextInput source="description" />
      <TextInput source="id" />
      <DateInput source="prefix" />
      <TextInput source="speech_audio_track" />
      <TextInput source="thumbnail" />
      <DateInput source="title" />
      <ReferenceArrayInput source="topic_ids" reference="topics">
        <TextInput source="id" />
      </ReferenceArrayInput>
      <TextInput source="updated_at" />
    </SimpleForm>
  </Edit>
);

export default StreamEdit;
