import { Datagrid, DateField, List, TextField, ListProps } from "react-admin";

const EpisodeList = (props: ListProps) => (
  <List {...props}>
    <Datagrid rowClick="edit">
      <TextField source="title" />
      <TextField source="description" />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default EpisodeList;
