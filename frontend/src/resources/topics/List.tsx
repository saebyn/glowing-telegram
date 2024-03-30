import { List, Datagrid, TextField } from "react-admin";

const TopicList = (props: any) => (
  <List {...props}>
    <Datagrid>
      <TextField source="title" />
      <TextField source="description" />
    </Datagrid>
  </List>
);

export default TopicList;
