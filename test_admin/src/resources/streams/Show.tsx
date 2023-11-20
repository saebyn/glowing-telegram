import {
  DateField,
  ReferenceArrayField,
  Show,
  ShowProps,
  SimpleShowLayout,
  TextField,
} from "react-admin";

const StreamShow = (props: ShowProps) => (
  <Show {...props}>
    <SimpleShowLayout>
      <DateField source="created_at" />
      <TextField source="description" />
      <TextField source="id" />
      <DateField source="prefix" />
      <TextField source="speech_audio_track" />
      <TextField source="thumbnail" />
      <DateField source="title" />
      <ReferenceArrayField source="topic_ids" reference="topics">
        <TextField source="id" />
      </ReferenceArrayField>
      <TextField source="updated_at" />
    </SimpleShowLayout>
  </Show>
);

export default StreamShow;
