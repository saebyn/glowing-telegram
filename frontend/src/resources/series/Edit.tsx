import {
  ArrayInput,
  BooleanInput,
  SimpleForm,
  TextInput,
  SimpleFormIterator,
} from "react-admin";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";
import Edit, { EditProps } from "../../Edit";
import YouTubeCategoryInput from "../../YouTubeCategoryInput";

const SeriesEdit = (props: EditProps) => (
  <Edit {...props}>
    <SimpleForm>
      <TitleInput source="title" required />
      <DescriptionInput source="description" />

      <TextInput source="thumbnail_url" />
      <TextInput source="playlist_id" />

      <BooleanInput source="notify_subscribers" />
      <YouTubeCategoryInput source="category" />
      <ArrayInput source="tags">
        <SimpleFormIterator>
          <TextInput source="" />
        </SimpleFormIterator>
      </ArrayInput>
    </SimpleForm>
  </Edit>
);

export default SeriesEdit;
