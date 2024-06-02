import {
  Create,
  SimpleForm,
  CreateProps,
  TextInput,
  ArrayInput,
  BooleanInput,
  SimpleFormIterator,
} from "react-admin";
import YouTubeCategoryInput from "../../YouTubeCategoryInput";

const EpisodeCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Stream">
    <SimpleForm>
      <TextInput source="title" required />

      <BooleanInput source="notify_subscribers" />
      <YouTubeCategoryInput source="category" />
      <ArrayInput source="tags">
        <SimpleFormIterator>
          <TextInput source="" />
        </SimpleFormIterator>
      </ArrayInput>
    </SimpleForm>
  </Create>
);

export default EpisodeCreate;
