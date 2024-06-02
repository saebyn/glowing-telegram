import {
  Create,
  SimpleForm,
  CreateProps,
  TextInput,
  ArrayInput,
  BooleanInput,
  SimpleFormIterator,
} from "react-admin";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";
import YouTubeCategoryInput from "../../YouTubeCategoryInput";

const SeriesCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Video Series">
    <SimpleForm>
      <TitleInput source="title" required />
      <DescriptionInput source="description" />

      <TextInput source="thumbnail_url" />
      <TextInput source="playlist_id" />

      <BooleanInput source="notify_subscribers" />
      <YouTubeCategoryInput source="category" />
      {/* editable array of strings as chips */}
      <ArrayInput source="tags">
        <SimpleFormIterator>
          <TextInput source="" />
        </SimpleFormIterator>
      </ArrayInput>
    </SimpleForm>
  </Create>
);

export default SeriesCreate;
