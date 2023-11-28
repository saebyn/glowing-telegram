import { Create, SimpleForm, CreateProps, TextInput } from "react-admin";

const StreamCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Video Clip">
    <SimpleForm>
      <TextInput source="title" required />
    </SimpleForm>
  </Create>
);

export default StreamCreate;
