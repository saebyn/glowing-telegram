import { useState } from "react";
import {
  Button,
  useRecordContext,
  useDataProvider,
  useRefresh,
  useInput,
} from "react-admin";
import { useFormContext } from "react-hook-form";
import { useMutation } from "react-query";
import { styled } from "@mui/material/styles";
import TextField from "@mui/material/TextField";
import { formatDuration, parseIntoSeconds } from "../../isoDuration";
import AsyncResultLoader from "./AsyncResultLoader";
import { TranscriptSegment } from "../../types";

const ScanButton = ({ label }: { label: string }) => {
  const record = useRecordContext();
  const refresh = useRefresh();
  const dataProvider = useDataProvider();

  const { mutate, isLoading } = useMutation<string | null>(() =>
    dataProvider.queueStreamTranscription({
      uris: record.video_clips.map((clip: any) => clip.uri),
      track: 2,
      initial_prompt: `
---
Date: ${record.prefix}
Title: ${record.title} on twitch.tv/saebyn
Description: ${record.description}
---


`,
      language: "en",

      stream_id: record.id,
    })
  );

  const queueTranscription = () => {
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
      onClick={queueTranscription}
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

interface StreamTranscriptInputProps {
  className?: string;
  source: string;
  taskUrlFieldName: string;
}

const StreamTranscriptInput = ({
  className,
  source,
  taskUrlFieldName,
}: StreamTranscriptInputProps) => {
  const [editing, setEditing] = useState<null | number>(null);
  const formContext = useFormContext();
  const {
    field: { value },
  } = useInput({ source });

  const transcriptSegments: TranscriptSegment[] = value || [];

  const onSave = (index: number, buffer: string) => {
    const newSegments = [...transcriptSegments];
    newSegments[index].text = buffer;
    formContext.setValue(source, newSegments, {
      shouldValidate: true,
      shouldDirty: true,
    });
  };

  return (
    <div className={className}>
      <ScanButton label="Transcribe" />
      <AsyncResultLoader source={source} taskUrlFieldName={taskUrlFieldName} />

      {transcriptSegments.map((segment: TranscriptSegment, index: number) => {
        return (
          <StreamTranscriptSegmentInput
            key={segment.start}
            segment={segment}
            index={index}
            editing={editing}
            setEditing={setEditing}
            onSave={onSave}
          />
        );
      })}
    </div>
  );
};

const StreamTranscriptSegmentInput = ({
  segment,
  index,
  editing,
  setEditing,
  onSave,
}: {
  segment: TranscriptSegment;
  index: number;
  editing: null | number;
  setEditing: (_index: null | number) => void;
  onSave: (_index: number, _text: string) => void;
}) => {
  const [buffer, setBuffer] = useState<null | string>(null);

  const segmentStart = parseIntoSeconds(segment.start);
  const segmentEnd = parseIntoSeconds(segment.end);

  return (
    <div
      key={segment.start}
      className={LabeledClasses.segment}
      onClick={() => {
        setEditing(index);
      }}
    >
      {index === editing ? (
        <TextField
          multiline={true}
          value={buffer || segment.text}
          onChange={(e) => {
            setBuffer(e.target.value);
          }}
          onBlur={() => {
            setEditing(null);
            if (buffer) {
              onSave(index, buffer);
              setBuffer(null);
            }
          }}
        />
      ) : (
        <span className={LabeledClasses.segmentText}>{segment.text}</span>
      )}

      <span className={LabeledClasses.segmentStart}>
        {formatDuration(segmentStart)}
      </span>
      <span className={LabeledClasses.segmentEnd}>
        {formatDuration(segmentEnd)}
      </span>
    </div>
  );
};

const PREFIX = "StreamTranscriptInput";

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

export default styled(StreamTranscriptInput)({
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
