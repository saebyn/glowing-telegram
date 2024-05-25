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
  BooleanField,
  ReferenceInput,
  SearchInput,
  FilterButton,
  TextInput,
  NullableBooleanInput,
} from "react-admin";
import TriggerRenderFileScanButton from "./TriggerRenderFileScanButton";
import UploadEpisodeToYoutubeButton from "./UploadEpisodeToYoutubeButton";
import { BulkExportButton } from "../../OTIOExporter/Exporter";

const ListActions = (props: any) => (
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
      <BooleanField source="is_published" />
      <DateField source="stream_date" />
      <DateField source="created_at" />
      <DateField source="updated_at" />
    </Datagrid>
  </List>
);

export default EpisodeList;
