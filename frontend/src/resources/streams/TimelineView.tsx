import { useState } from "react";
import {
  Button,
  useDataProvider,
  useNotify,
  useRecordContext,
} from "react-admin";
import { useMutation } from "react-query";
import {
  convertSecondsToISODuration,
  parseIntoSeconds,
} from "../../isoDuration";

import Timeline, { DataStreamDataElement } from "../../Timeline/StreamTimeline";

import { styled } from "@mui/material/styles";
import { Segment } from "../../Timeline/SegmentSelector";

interface TimelineViewProps {
  className?: string;
}

const TimelineView = ({ className }: TimelineViewProps) => {
  const record = useRecordContext();

  const start = 0;
  const end = parseIntoSeconds(record.duration);

  const silenceDetectionSegments: DataStreamDataElement[] =
    record.silence_segments?.map((segment: any) => ({
      start: parseIntoSeconds(segment.start),
      end: parseIntoSeconds(segment.end),
      density: 1,
    })) || [];

  const initialSegments: Segment[] = periodsBetweenSegments(
    silenceDetectionSegments,
    end - start
  )
    .map((segment, index) => ({
      id: index,
      start: segment.start,
      end: segment.end,
    }))
    // Filter out zeroish-length segments
    .filter((segment) =>  segment.end - segment.start > 0.1);

  const [segments, setSegments] = useState<Segment[]>(initialSegments);

  const handleUpdateSegments = (segments: any) => {
    setSegments(segments);
  };

  const handleResetSegments = () => {
    setSegments(initialSegments);
  };

  if (!record) {
    return <>Loading...</>;
  }

  return (
    <div className={className}>
      <BulkCreateEpisodesButton
        label="Bulk Create Episodes"
        segments={segments}
      />

      <Timeline
        segments={segments}
        onUpdate={handleUpdateSegments}
        onReset={handleResetSegments}
        start={start}
        end={end}
        dataStreams={[
          {
            name: "Silence Detection",
            data: silenceDetectionSegments,
            color: [0, 0, 255],
          },
        ]}
      />
    </div>
  );
};

const PREFIX = "TimelineView";

export const LabeledClasses = {
  root: `${PREFIX}-root`,
  scanButton: `${PREFIX}-scanButton`,
  taskStatus: `${PREFIX}-taskStatus`,
  asyncResultLoader: `${PREFIX}-asyncResultLoader`,
};

export default styled(TimelineView)({
  width: "100%",
});

function periodsBetweenSegments(
  segments: DataStreamDataElement[],
  totalDuration: number
): DataStreamDataElement[] {
  const periods: DataStreamDataElement[] = [];

  const paddedSegments = [
    { start: 0, end: 0 },
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
