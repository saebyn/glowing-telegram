import { List, Datagrid, TextField, ListProps } from "react-admin";

const TopicList = (props: ListProps) => (
  <List {...props}>
    <Datagrid>
      <TextField source="title" />
      <TextField source="description" />
    </Datagrid>
  </List>
);

export default TopicList;
