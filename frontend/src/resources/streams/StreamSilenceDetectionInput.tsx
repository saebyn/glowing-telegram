import { useState } from "react";
import {
  Button,
  useRecordContext,
  useDataProvider,
  useRefresh,
  ArrayInput,
  SimpleFormIterator,
} from "react-admin";
import { useMutation } from "@tanstack/react-query";
import { styled } from "@mui/material/styles";
import AsyncResultLoader from "./AsyncResultLoader";
import { DurationInput } from "../../DurationInput";

import { TextField } from "@mui/material";

const ScanButton = ({ label }: { label: string }) => {
  const record = useRecordContext();
  const refresh = useRefresh();
  const dataProvider = useDataProvider();

  const [track, setTrack] = useState(2);
  const [duration, setDuration] = useState(30);

  const { mutate, isPending } = useMutation<string | null>({
    mutationKey: ["queueStreamSilenceDetection", record?.id],
    mutationFn: () =>
      dataProvider.queueStreamSilenceDetection({
        task_title: `Silence Detection for ${record?.title}`,
        uris: record?.video_clips.map((clip: any) => clip.uri),
        track,
        duration,
        stream_id: record?.id,
      }),
  });

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
        disabled={isPending}
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
  return (
    <div className={className}>
      <ScanButton label="Detect Silences" />
      <AsyncResultLoader source={source} taskUrlFieldName={taskUrlFieldName} />

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
