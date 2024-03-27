import {
  Datagrid,
  DateField,
  List,
  TextField,
  ListProps,
  CreateButton,
  TopToolbar,
} from "react-admin";
import TriggerRenderFileScanButton from "./TriggerRenderFileScanButton";

const ListActions = () => (
  <TopToolbar>
    <CreateButton />
    <TriggerRenderFileScanButton />
  </TopToolbar>
);

const EpisodeList = (props: ListProps) => (
  <List {...props} actions={<ListActions />}>
    <Datagrid rowClick="edit">
      <TextField source="title" />
      <TextField source="description" />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default EpisodeList;
