import {
  useRecordContext,
  useNotify,
  useDataProvider,
  Button,
  useReference,
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
  const streamRecord = useRecordContext();
  const notify = useNotify();
  const dataProvider = useDataProvider();

  const {
    referenceRecord: series,
    isLoading: isLoadingSeries,
    error: errorSeries,
  } = useReference({ reference: "series", id: streamRecord.series_id });

  const { mutate, isLoading, error } = useMutation<
    string | null,
    any,
    DataStreamDataElement[]
  >((segments) => {
    if (isLoadingSeries) {
      return;
    }

    const baseEpIndex = (series?.max_episode_order_index || 0) + 1;

    return dataProvider.bulkCreate(
      "episodes",
      segments.map((segment, index) => ({
        stream_id: streamRecord.id,
        series_id: streamRecord.series_id,
        order_index: baseEpIndex + index,
        title: `${streamRecord.title} - Episode ${baseEpIndex + index}`,
        tracks: [
          {
            start: convertSecondsToISODuration(segment.start),
            end: convertSecondsToISODuration(segment.end),
          },
        ],
        notify_subscribers: series?.notify_subscribers,
        category: series?.category,
        tags: series?.tags,
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

  if (errorSeries) {
    return <div>{errorSeries}</div>;
  }

  if (error) {
    return <div>{error}</div>;
  }

  return (
    <Button
      disabled={isLoading || isLoadingSeries}
      label={`Start ${label}`}
      onClick={bulkCreateEpisodes}
    />
  );
};

export default BulkCreateEpisodesButton;
