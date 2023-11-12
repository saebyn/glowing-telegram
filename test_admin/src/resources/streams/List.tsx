import {
  Datagrid,
  DateField,
  List,
  ReferenceArrayField,
  TextField,
  ImageField,
} from "react-admin";

const StreamList = () => (
  <List>
    <Datagrid rowClick="edit">
      <DateField source="title" />
      <ImageField source="thumbnail" />
      <ReferenceArrayField source="topic_ids" reference="topics" />
      <DateField source="created_at" />
      <TextField source="updated_at" />
    </Datagrid>
  </List>
);

export default StreamList;
