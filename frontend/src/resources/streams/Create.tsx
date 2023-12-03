import { Create, SimpleForm, CreateProps, TextInput } from "react-admin";

const StreamCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Stream">
    <SimpleForm>
      <TextInput source="title" required />
      <TextInput
        source="prefix"
        required
        helperText="The prefix is used to identify related video clips for this stream. It's typically in the format YYYY-MM-DD."
        inputProps={{ pattern: "[0-9]{4}-[0-9]{2}-[0-9]{2}" }}
      />
    </SimpleForm>
  </Create>
);

export default StreamCreate;
