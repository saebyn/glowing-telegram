import {
  Datagrid,
  DateField,
  List,
  TextField,
  ListProps,
  BulkExportButton,
} from "react-admin";

const BulkActionButtons = () => (
  <>
    <BulkExportButton />
  </>
);

const EpisodeList = (props: ListProps) => (
  <List {...props}>
    <Datagrid rowClick="edit" bulkActionButtons={<BulkActionButtons />}>
      <TextField source="title" />
      <TextField source="description" />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default EpisodeList;
