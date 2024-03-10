import { Create, SimpleForm, CreateProps, TextInput } from "react-admin";

const EpisodeCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Stream">
    <SimpleForm>
      <TextInput source="title" required />
    </SimpleForm>
  </Create>
);

export default EpisodeCreate;
