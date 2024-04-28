import { SimpleForm, TextInput } from "react-admin";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";
import Edit, { EditProps } from "../../Edit";

const SeriesEdit = (props: EditProps) => (
  <Edit {...props}>
    <SimpleForm>
      <TitleInput source="title" required />
      <DescriptionInput source="description" />

      <TextInput source="thumbnail_url" />
    </SimpleForm>
  </Edit>
);

export default SeriesEdit;
