import {
  Datagrid,
  DateField,
  List,
  ReferenceArrayField,
  TextField,
  ListProps,
} from "react-admin";
import ThumbnailField from "../../ThumbnailField";

const StreamList = (props: ListProps) => (
  <List {...props}>
    <Datagrid rowClick="edit">
      <DateField source="stream_date" />
      <TextField source="prefix" />
      <TextField source="title" />
      <ThumbnailField source="thumbnail" width={100} height={100} />
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
