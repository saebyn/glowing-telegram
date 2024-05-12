import { Create, SimpleForm, CreateProps, TextInput } from "react-admin";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";

const SeriesCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Video Series">
    <SimpleForm>
      <TitleInput source="title" required />
      <DescriptionInput source="description" />

      <TextInput source="thumbnail_url" />
      <TextInput source="playlist_id" />
    </SimpleForm>
  </Create>
);

export default SeriesCreate;
