import {
  Datagrid,
  DateField,
  List,
  TextField,
  ListProps,
  CloneButton,
  TextInput,
  Filter,
} from "react-admin";
import ThumbnailField from "../../ThumbnailField";

// TODO add q to the crud api
const StreamsFilter = (props: any) => (
  <Filter {...props}>
    <TextInput label="Search" source="q" alwaysOn />
  </Filter>
);

const StreamList = (props: ListProps) => (
  <List {...props} filters={<StreamsFilter />}>
    <Datagrid rowClick="edit">
      <DateField source="stream_date" />
      <TextField source="prefix" />
      <TextField source="title" />
      <ThumbnailField source="thumbnail" width={100} height={100} />
      <DateField source="created_at" />
      <DateField source="updated_at" />
      <CloneButton />
    </Datagrid>
  </List>
);

export default StreamList;
