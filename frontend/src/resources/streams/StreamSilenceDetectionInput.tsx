//import { useState } from "react";
import {
  Button,
  useRecordContext,
  useDataProvider,
  useRefresh,
  useNotify,
  ArrayInput,
  SimpleFormIterator,
} from "react-admin";
import { useMutation } from "react-query";
import { styled } from "@mui/material/styles";
import AsyncResultLoader from "./AsyncResultLoader";
import Timeline from "../../Timeline";
import { parseIntoSeconds, toISO8601Duration } from "../../isoDuration";
import { DurationInput } from "../../DurationInput";
import { useState } from "react";
import { TextField } from "@mui/material";

const ScanButton = ({ label }: { label: string }) => {
  const record = useRecordContext();
  const refresh = useRefresh();
  const dataProvider = useDataProvider();

  const [track, setTrack] = useState(2);
  const [duration, setDuration] = useState(30);

  const { mutate, isLoading } = useMutation<string | null>(() =>
    dataProvider.queueStreamSilenceDetection({
      task_title: `Silence Detection for ${record.title}`,
      uris: record.video_clips.map((clip: any) => clip.uri),
      track,
      duration,
      stream_id: record.id,
    })
  );

  const queueSilenceDetection = () => {
    mutate(void 0, {
      onSuccess: () => {
        refresh();
      },
    });
  };

  return (
    <>
      <Button
        disabled={isLoading}
        label={`Start ${label}`}
        onClick={queueSilenceDetection}
      />
      <TextField
        label="Track"
        type="number"
        value={track}
        onChange={(e) => setTrack(parseInt(e.target.value, 10))}
      />
      <TextField
        label="Duration"
        type="number"
        value={duration}
        onChange={(e) => setDuration(parseInt(e.target.value, 10))}
      />
    </>
  );
};

interface Segment {
  start: string;
  end: string;
}

function periodsBetweenSegments(
  segments: Segment[],
  totalDuration: string
): Segment[] {
  const periods: Segment[] = [];

  const paddedSegments = [
    { start: "PT0S", end: "PT0S" },
    ...segments,
    { start: totalDuration, end: totalDuration },
  ];

  for (let i = 0; i < paddedSegments.length - 1; i++) {
    periods.push({
      start: paddedSegments[i].end,
      end: paddedSegments[i + 1].start,
    });
  }

  return periods;
}

const BulkCreateEpisodesButton = ({
  label,
  segments,
}: {
  label: string;
  segments: Segment[];
}) => {
  const record = useRecordContext();
  const notify = useNotify();
  const dataProvider = useDataProvider();

  const { mutate, isLoading } = useMutation<
    string | null,
    unknown,
    {
      segments: Segment[];
      totalDuration: string;
    }
  >(({ segments, totalDuration }) => {
    return dataProvider.bulkCreate(
      "episodes",
      periodsBetweenSegments(segments, totalDuration).map((segment, index) => ({
        stream_id: record.id,
        title: `${record.title} - Episode ${index + 1}`,
        tracks: [
          {
            start: segment.start,
            end: segment.end,
          },
        ],
      }))
    );
  });

  const bulkCreateEpisodes = () => {
    const totalDuration = toISO8601Duration({
      hours: 0,
      minutes: 0,
      milliseconds: 0,
      seconds: record.video_clips.reduce(
        (acc: number, clip: any) => acc + parseIntoSeconds(clip.duration),
        0
      ),
    });
    mutate(
      { segments, totalDuration },
      {
        onSuccess: () => {
          // tell the user that the episodes were created
          notify("Episodes created");
        },
      }
    );
  };

  return (
    <Button
      disabled={isLoading}
      label={`Start ${label}`}
      onClick={bulkCreateEpisodes}
    />
  );
};

export const TaskStatus = ({
  taskStatus,
}: {
  taskStatus: string | null | undefined;
}) => {
  switch (taskStatus) {
    case "Queued":
      return <p>Task queued</p>;
    case "Processing":
      return <p>Task running</p>;
    case "Complete":
      return <p>Task completed</p>;
    case "Failed":
      return <p>Task failed</p>;
    default:
      return null;
  }
};

interface StreamSilenceDetectionInputProps {
  className?: string;
  source: string;
  taskUrlFieldName: string;
}

const StreamSilenceDetectionInput = ({
  className,
  source,
  taskUrlFieldName,
  ...props
}: StreamSilenceDetectionInputProps) => {
  const record = useRecordContext();

  const [selectedSegmentIndices, setSelectedSegmentIndices] = useState<
    Map<number, boolean>
  >(new Map());

  const handleSelectedSegmentIndicesChange = (index: number) => {
    setSelectedSegmentIndices((selectedSegmentIndices) => {
      if (selectedSegmentIndices.get(index)) {
        selectedSegmentIndices.delete(index);
        return selectedSegmentIndices;
      } else {
        selectedSegmentIndices.set(index, true);
        return selectedSegmentIndices;
      }
    });
  };

  const handleUpdateSegment = (
    index: number,
    segment: {
      start: number;
      end: number;
    }
  ) => {
    console.log(index, segment);
  };

  if (!record) {
    return <>Loading...</>;
  }

  const silenceDetectionSegments: Segment[] = record[source] || [];
  const selectedSegments = silenceDetectionSegments.filter((_, index) =>
    selectedSegmentIndices.get(index)
  );

  return (
    <div className={className}>
      <ScanButton label="Detect Silences" />
      <AsyncResultLoader source={source} taskUrlFieldName={taskUrlFieldName} />
      <BulkCreateEpisodesButton
        label="Bulk Create Episodes"
        segments={selectedSegments}
      />

      <Timeline
        duration={(record.video_clips || []).reduce(
          (acc: number, clip: any) => acc + parseIntoSeconds(clip.duration),
          0
        )}
        onUpdateSegment={handleUpdateSegment}
        onToggleSegment={handleSelectedSegmentIndicesChange}
        selectedSegmentIndices={selectedSegmentIndices}
        segments={silenceDetectionSegments.map((segment: any) => {
          return {
            start: parseIntoSeconds(segment.start),
            end: parseIntoSeconds(segment.end),
          };
        })}
      />

      <ArrayInput source={source} {...props}>
        <SimpleFormIterator>
          <DurationInput source="start" />
          <DurationInput source="end" />
        </SimpleFormIterator>
      </ArrayInput>
    </div>
  );
};

const PREFIX = "StreamSilenceDetectionInput";

export const LabeledClasses = {
  root: `${PREFIX}-root`,
  scanButton: `${PREFIX}-scanButton`,
  taskStatus: `${PREFIX}-taskStatus`,
  asyncResultLoader: `${PREFIX}-asyncResultLoader`,
};

export default styled(StreamSilenceDetectionInput)({
  width: "100%",
});
