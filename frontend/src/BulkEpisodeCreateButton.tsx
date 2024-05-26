import {
  useRecordContext,
  useNotify,
  useDataProvider,
  Button,
} from "react-admin";
import { useMutation } from "react-query";
import { convertSecondsToISODuration } from "./isoDuration";
import { DataStreamDataElement } from "./types";

const BulkCreateEpisodesButton = ({
  label,
  segments,
}: {
  label: string;
  segments: DataStreamDataElement[];
}) => {
  const record = useRecordContext();
  const notify = useNotify();
  const dataProvider = useDataProvider();

  const { mutate, isLoading } = useMutation<
    string | null,
    unknown,
    DataStreamDataElement[]
  >((segments) => {
    return dataProvider.bulkCreate(
      "episodes",
      segments.map((segment, index) => ({
        stream_id: record.id,
        series_id: record.series_id,
        title: `${record.title} - Episode ${index + 1}`,
        tracks: [
          {
            start: convertSecondsToISODuration(segment.start),
            end: convertSecondsToISODuration(segment.end),
          },
        ],
      }))
    );
  });

  const bulkCreateEpisodes = () => {
    mutate(segments, {
      onSuccess: () => {
        // tell the user that the episodes were created
        notify("Episodes created");
      },
    });
  };

  return (
    <Button
      disabled={isLoading}
      label={`Start ${label}`}
      onClick={bulkCreateEpisodes}
    />
  );
};

export default BulkCreateEpisodesButton;
