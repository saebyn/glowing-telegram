import {
  Datagrid,
  DateField,
  List,
  ReferenceArrayField,
  TextField,
  ImageField,
  ListProps,
} from "react-admin";

const StreamList = (props: ListProps) => (
  <List {...props}>
    <Datagrid rowClick="edit">
      <TextField source="prefix" />
      <TextField source="title" />
      <ImageField source="thumbnail" sortable={false} />
      <ReferenceArrayField
        source="topic_ids"
        reference="topics"
        sortable={false}
      />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default StreamList;
