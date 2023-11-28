import { Datagrid, DateField, List, TextField, ListProps } from "react-admin";

const StreamList = (props: ListProps) => (
  <List {...props}>
    <Datagrid rowClick="edit">
      <TextField source="title" />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default StreamList;
