import { useState } from "react";
import {
  Button,
  useDataProvider,
  useNotify,
  useRecordContext,
} from "react-admin";
import { useMutation } from "react-query";
import { parseIntoSeconds, toISO8601Duration } from "../../isoDuration";

import Timeline from "../../Timeline/StreamTimeline";

import { styled } from "@mui/material/styles";

interface TimelineViewProps {
  className?: string;
}

const TimelineView = ({ className }: TimelineViewProps) => {
  const record = useRecordContext();
  const [selectedSegmentIndices, setSelectedSegmentIndices] = useState<
    Map<number, boolean>
  >(new Map());

  if (!record) {
    return <>Loading...</>;
  }

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
    alert("Implement me!");
    console.log(index, segment);
  };

  if (!record) {
    return <>Loading...</>;
  }

  const silenceDetectionSegments: Segment[] =
    record.silence_detection_segments?.map((segment: any) => {
      return {
        start: parseIntoSeconds(segment.start),
        end: parseIntoSeconds(segment.end),
      };
    }) || [];

  const selectedSegments = silenceDetectionSegments.filter((_, index) =>
    selectedSegmentIndices.get(index)
  );

  return (
    <div className={className}>
      <BulkCreateEpisodesButton
        label="Bulk Create Episodes"
        segments={selectedSegments}
      />

      <Timeline />
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
