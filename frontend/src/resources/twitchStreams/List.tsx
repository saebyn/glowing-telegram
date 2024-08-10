import {
  TextField,
  Datagrid,
  InfiniteList,
  SimpleShowLayout,
  Button,
  useRefresh,
  useNotify,
  useDataProvider,
  useListContext,
  useUnselectAll,
} from "react-admin";
import ImportIcon from "@mui/icons-material/ImportExport";
import ThumbnailField from "../../ThumbnailField";

const StreamPanel = () => (
  <SimpleShowLayout>
    <TextField source="id" />

    <TextField source="language" />
    <TextField source="type" />

    <TextField source="url" />
    <TextField source="stream_id" />
    <TextField source="viewable" />
  </SimpleShowLayout>
);

interface TwitchStreamData {
  id: string;
  title: string;
  duration: string;
  thumbnail_url: string;
  stream_id: string;
  created_at: string;
}

const BulkActionButtons = () => {
  const refresh = useRefresh();
  const notify = useNotify();
  const dataProvider = useDataProvider();
  const { selectedIds, data } = useListContext<TwitchStreamData>();
  const unselectAll = useUnselectAll("twitchStreams");

  const handleImport = () => {
    if (!selectedIds.length) {
      return;
    }

    if (data === undefined) {
      return;
    }

    // Notify the user that the import started
    notify("Importing streams");

    // get the selected records
    const selectedRecords = data
      .filter((record) => selectedIds.includes(record.id))
      .map(({ title, duration, thumbnail_url, stream_id, created_at }) => {
        // convert the duration in the format "HHhMMmSSs" to the ISO 8601 format
        const durationParts = duration.match(/(\d+)h(\d+)m(\d+)s/);

        if (!durationParts) {
          throw new Error(`Invalid duration format: ${duration}`);
        }

        const durationISO = `PT${durationParts[1]}H${durationParts[2]}M${durationParts[3]}S`;

        const createdAt = new Date(created_at);
        return {
          title,
          duration: durationISO,
          thumbnail: thumbnail_url,
          stream_id,
          stream_date: created_at,
          stream_platform: "twitch",

          // the date portion of the created_at field
          prefix: `${createdAt.getFullYear()}-${(createdAt.getMonth() + 1)
            .toString()
            .padStart(2, "0")}-${createdAt
            .getDate()
            .toString()
            .padStart(2, "0")}`,
        };
      });

    // Perform the import
    dataProvider.importStreams(selectedRecords).then(() => {
      notify("Streams imported");

      // Unselect all records
      unselectAll();

      // Refresh the list
      refresh();
    });
  };

  return (
    <Button label="Import" onClick={handleImport} startIcon={<ImportIcon />} />
  );
};

const TwitchStreamsList = () => (
  <InfiniteList>
    <Datagrid
      expand={<StreamPanel />}
      expandSingle
      bulkActionButtons={<BulkActionButtons />}
    >
      <ThumbnailField source="thumbnail_url" label="Thumbnail" />

      <TextField source="title" />

      <TextField source="view_count" />

      <TextField source="created_at" />
      <TextField source="published_at" />

      <TextField source="duration" />
    </Datagrid>
  </InfiniteList>
);

export default TwitchStreamsList;
