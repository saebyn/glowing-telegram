import {
  Datagrid,
  DateField,
  List,
  TextField,
  ListProps,
  CreateButton,
  TopToolbar,
  ReferenceField,
  NumberField,
  ReferenceInput,
  SearchInput,
  FilterButton,
  TextInput,
  NullableBooleanInput,
  ListActionsProps,
} from "react-admin";
import TriggerRenderFileScanButton from "./TriggerRenderFileScanButton";
import UploadEpisodeToYoutubeButton from "./UploadEpisodeToYoutubeButton";
import { BulkExportButton } from "../../OTIOExporter/Exporter";

const ListActions = (props: ListActionsProps) => (
  <TopToolbar {...props}>
    <FilterButton />
    <CreateButton />
    <TriggerRenderFileScanButton />
  </TopToolbar>
);

const BulkActionButtons = () => (
  <>
    <UploadEpisodeToYoutubeButton />

    <BulkExportButton />
  </>
);

const episodeFilters = [
  // eslint-disable-next-line react/jsx-key
  <SearchInput source="title" alwaysOn />,
  // eslint-disable-next-line react/jsx-key
  <ReferenceInput source="series_id" reference="series" />,
  // eslint-disable-next-line react/jsx-key
  <ReferenceInput source="stream_id" reference="streams" />,
  // eslint-disable-next-line react/jsx-key
  <TextInput source="stream_name" />,
  // eslint-disable-next-line react/jsx-key
  <NullableBooleanInput source="has_youtube_video_id" />,
  // eslint-disable-next-line react/jsx-key
  <NullableBooleanInput source="is_published" />,
];

const EpisodeList = (props: ListProps) => (
  <List {...props} filters={episodeFilters} actions={<ListActions />}>
    <Datagrid rowClick="edit" bulkActionButtons={<BulkActionButtons />}>
      <TextField source="title" />
      <ReferenceField source="series_id" reference="series">
        <TextField source="title" />
      </ReferenceField>
      <NumberField source="order_index" />

      <DateField source="stream_date" />
    </Datagrid>
  </List>
);

export default EpisodeList;
