import { Create, CreateProps, SimpleForm } from "react-admin";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";

const TopicCreate = (props: CreateProps) => (
  <Create {...props}>
    <SimpleForm>
      <TitleInput source="title" />
      <DescriptionInput source="description" />
    </SimpleForm>
  </Create>
);

export default TopicCreate;
