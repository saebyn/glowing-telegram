import {
  Create,
  SimpleForm,
  CreateProps,
  TextInput,
  ArrayInput,
  BooleanInput,
  SimpleFormIterator,
  ReferenceInput,
  SelectInput,
  NumberInput,
} from "react-admin";
import { DurationInput } from "../../DurationInput";
import YouTubeCategoryInput from "../../YouTubeCategoryInput";

const EpisodeCreate = (props: CreateProps) => (
  <Create {...props} title="Create an Episode">
    <SimpleForm>
      <TextInput source="title" required />

      <TextInput source="stream_id" isRequired={true} />

      <ReferenceInput source="series_id" reference="series">
        <SelectInput optionText="title" />
      </ReferenceInput>

      <NumberInput source="order_index" />

      <TextInput source="youtube_video_id" />

      <BooleanInput source="is_published" />

      <BooleanInput source="notify_subscribers" />
      <YouTubeCategoryInput source="category" />
      <ArrayInput source="tags">
        <SimpleFormIterator>
          <TextInput source="" />
        </SimpleFormIterator>
      </ArrayInput>

      <ArrayInput source="tracks">
        <SimpleFormIterator>
          <DurationInput source="start" />
          <DurationInput source="end" />
        </SimpleFormIterator>
      </ArrayInput>
    </SimpleForm>
  </Create>
);

export default EpisodeCreate;
