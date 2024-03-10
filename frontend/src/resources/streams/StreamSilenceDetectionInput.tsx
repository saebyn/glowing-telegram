//import { useState } from "react";
import {
  Button,
  useRecordContext,
  useDataProvider,
  useRefresh,
  //useInput,
  ArrayInput,
  SimpleFormIterator,
  TextInput,
} from "react-admin";
//import { useFormContext } from "react-hook-form";
import { useMutation } from "react-query";
import { styled } from "@mui/material/styles";
//import { formatDuration, parseISODuration } from "../../isoDuration";
import AsyncResultLoader from "./AsyncResultLoader";
import Timeline from "../../Timeline";
import { parseISODuration } from "../../isoDuration";

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

  const queueSilenceDetection = () => {
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
      onClick={queueSilenceDetection}
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

/*interface SilenceDetectionSegment {
  start: string;
  end: string;
}*/

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
  const silenceDetectionSegments = record[source] || [];
  /*const [editing, setEditing] = useState<null | number>(null);
  const formContext = useFormContext();
  */

  /*
  const onSave = (index: number, buffer: string) => {
    const newSegments = [...silenceDetectionSegments];
    newSegments[index].text = buffer;
    formContext.setValue(source, newSegments, {
      shouldValidate: true,
      shouldDirty: true,
    });
  };*/

  return (
    <div className={className}>
      <ScanButton label="Detect Silences" />
      <AsyncResultLoader source={source} taskUrlFieldName={taskUrlFieldName} />

      <Timeline
        duration={record.video_clips.reduce(
          (acc: number, clip: any) => acc + parseISODuration(clip.duration),
          0
        )}
        segments={silenceDetectionSegments.map((segment: any) => {
          return {
            start: parseISODuration(segment.start),
            end: parseISODuration(segment.end),
          };
        })}
      />

      <ArrayInput source={source} {...props}>
        <SimpleFormIterator>
          <TextInput source="start" />
          <TextInput source="end" />
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
