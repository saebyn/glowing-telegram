import {
  Create,
  SimpleForm,
  CreateProps,
  TextInput,
  DateInput,
} from "react-admin";

const StreamCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Stream">
    <SimpleForm>
      <TextInput source="title" required />
      <DateInput source="prefix" required />
    </SimpleForm>
  </Create>
);

export default StreamCreate;
