import {
  TextField,
  Datagrid,
  InfiniteList,
  FunctionField,
  SimpleShowLayout,
  Button,
  useRefresh,
  useNotify,
  useDataProvider,
  useListContext,
  useUnselectAll,
} from "react-admin";
import ImportIcon from "@mui/icons-material/ImportExport";

const ThumbnailField = ({ width, height }: any) => {
  const widthValue = width || 100;
  const heightValue = height || 100;

  return (
    <FunctionField
      render={(record: any) => {
        const thumbnailUrl = record.thumbnail_url
          .replace("%{width}", widthValue)
          .replace("%{height}", heightValue);
        return (
          <img
            src={thumbnailUrl}
            alt={record.title}
            width={widthValue}
            height={heightValue}
          />
        );
      }}
    />
  );
};

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

const BulkActionButtons = () => {
  const refresh = useRefresh();
  const notify = useNotify();
  const dataProvider = useDataProvider();
  const { selectedIds, data } = useListContext();
  const unselectAll = useUnselectAll("twitchStreams");

  const handleImport = () => {
    // Notify the user that the import started
    notify("Importing streams");

    // get the selected records
    const selectedRecords = data
      .filter((record: any) => selectedIds.includes(record.id))
      .map(({ title, duration, thumbnail_url, stream_id, created_at }: any) => {
        const createdAt = new Date(created_at);
        return {
          title,
          duration,
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
