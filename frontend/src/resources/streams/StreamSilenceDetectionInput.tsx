import { useState } from "react";
import {
  Button,
  useRecordContext,
  useDataProvider,
  useRefresh,
} from "react-admin";
import { useFormContext } from "react-hook-form";
import { useMutation } from "react-query";
import { styled } from "@mui/material/styles";
import { formatDuration, parseISODuration } from "../../isoDuration";
import AsyncResultLoader from "./AsyncResultLoader";

const ScanButton = ({ label }: { label: string }) => {
  const record = useRecordContext();
  const refresh = useRefresh();
  const dataProvider = useDataProvider();

  const { mutate, isLoading } = useMutation<string | null>(() =>
    dataProvider.queueStreamSilenceDetection({
      uris: record.video_clips.map((clip: any) => clip.uri),
      track: 2,
      stream_id: record.id,
    })
  );

  const queueSilenceDetectionion = () => {
    mutate(void 0, {
      onSuccess: () => {
        refresh();
      },
    });
  };

  return (
    <Button
      disabled={isLoading}
      label={`Start ${label}`}
      onClick={queueSilenceDetectionion}
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

interface SilenceDetectionSegment {
  start: string;
  end: string;
}

interface StreamSilenceDetectionInputProps {
  className?: string;
  source: string;
  taskUrlFieldName: string;
}

const StreamSilenceDetectionInput = ({
  className,
  source,
  taskUrlFieldName,
}: StreamSilenceDetectionInputProps) => {
  const record = useRecordContext();
  const [editing, setEditing] = useState<null | number>(null);
  const formContext = useFormContext();
  const silenceDetectionSegments = record[source] || [];

  const onSave = (index: number, buffer: string) => {
    const newSegments = [...silenceDetectionSegments];
    newSegments[index].text = buffer;
    formContext.setValue(source, newSegments, {
      shouldValidate: true,
      shouldDirty: true,
    });
  };

  return (
    <div className={className}>
      <ScanButton label="Detect Silences" />
      <AsyncResultLoader source={source} taskUrlFieldName={taskUrlFieldName} />

      {silenceDetectionSegments.map(
        (segment: SilenceDetectionSegment, index: number) => {
          return (
            <StreamSilenceDetectionSegmentInput
              key={segment.start}
              segment={segment}
              index={index}
              editing={editing}
              setEditing={setEditing}
              onSave={onSave}
            />
          );
        }
      )}
    </div>
  );
};

const StreamSilenceDetectionSegmentInput = ({
  segment,
  index,
  setEditing,
}: {
  segment: SilenceDetectionSegment;
  index: number;
  editing: null | number;
  setEditing: (_index: null | number) => void;
  onSave: (_index: number, _text: string) => void;
}) => {
  const segmentStart = parseISODuration(segment.start);
  const segmentEnd = parseISODuration(segment.end);

  return (
    <div
      key={segment.start}
      className={LabeledClasses.segment}
      onClick={() => {
        setEditing(index);
      }}
    >
      <span className={LabeledClasses.segmentStart}>
        {formatDuration(segmentStart)}
      </span>
      <span className={LabeledClasses.segmentEnd}>
        {formatDuration(segmentEnd)}
      </span>
    </div>
  );
};

const PREFIX = "StreamSilenceDetectionInput";

export const LabeledClasses = {
  root: `${PREFIX}-root`,
  scanButton: `${PREFIX}-scanButton`,
  taskStatus: `${PREFIX}-taskStatus`,
  asyncResultLoader: `${PREFIX}-asyncResultLoader`,
  segment: `${PREFIX}-segment`,
  segmentText: `${PREFIX}-segmentText`,
  segmentStart: `${PREFIX}-segmentStart`,
  segmentEnd: `${PREFIX}-segmentEnd`,
};

export default styled(StreamSilenceDetectionInput)({
  [`& .${LabeledClasses.segment}`]: {
    display: "grid",
    marginBottom: "8px",

    gridTemplateColumns: "100px 1fr",
    gridTemplateAreas: `"start text"
                        "end text"`,
  },

  [`& .${LabeledClasses.segmentText}`]: {
    gridArea: "text",
  },
  [`& .${LabeledClasses.segmentStart}`]: {
    gridArea: "start",
  },
  [`& .${LabeledClasses.segmentEnd}`]: {
    gridArea: "end",
  },
});
