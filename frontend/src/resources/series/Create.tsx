import { Create, SimpleForm, CreateProps } from "react-admin";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";

const SeriesCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Video Series">
    <SimpleForm>
      <TitleInput source="title" required />
      <DescriptionInput source="description" />
    </SimpleForm>
  </Create>
);

export default SeriesCreate;
