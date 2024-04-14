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
import UploadEpisodeToYoutubeButton from "./UploadEpisodeToYoutubeButton";

const ListActions = () => (
  <TopToolbar>
    <CreateButton />
    <TriggerRenderFileScanButton />
  </TopToolbar>
);

const BulkActionButtons = () => (
  <>
    <UploadEpisodeToYoutubeButton />
  </>
);

const EpisodeList = (props: ListProps) => (
  <List {...props} actions={<ListActions />}>
    <Datagrid rowClick="edit" bulkActionButtons={<BulkActionButtons />}>
      <TextField source="title" />
      <TextField source="description" />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default EpisodeList;
