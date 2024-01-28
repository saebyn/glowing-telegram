import { useState } from "react";
import {
  Button,
  ArrayInput,
  SimpleFormIterator,
  useRecordContext,
  useDataProvider,
  useRefresh,
  useNotify,
} from "react-admin";
import { useFormContext } from "react-hook-form";
import { useMutation } from "react-query";

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

const TaskStatus = ({
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

interface Segment {
  start: string;
  end: string;
  text: string;
}

interface Task {
  status: "Queued" | "Processing" | "Complete" | "Failed";
  id: string;
  data: Segment[];
}

const AsyncResultLoader = ({
  source,
  taskUrlFieldName,
}: {
  source: string;
  taskUrlFieldName: string;
}) => {
  const record = useRecordContext();
  const dataProvider = useDataProvider();
  const formContext = useFormContext();
  const notify = useNotify();

  const [task, setTask] = useState<Task | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  if (!record || !record[taskUrlFieldName]) {
    return null;
  }

  const dataReady = task?.status === "Complete";

  const checkStatus = async () => {
    setIsLoading(true);

    try {
      const taskData = await dataProvider.getTranscriptionTask(
        record[taskUrlFieldName]
      );
      setTask(taskData);
    } catch (e) {
      notify(`Failed to get task: ${e}`, { type: "error" });
    }

    setIsLoading(false);
  };

  const loadData = () => {
    const values: Segment[] = task?.data || [];

    formContext.setValue(source, values, {
      shouldValidate: true,
      shouldDirty: true,
    });

    formContext.setValue(taskUrlFieldName, null, {
      shouldValidate: true,
      shouldDirty: true,
    });
  };

  return (
    <>
      <Button
        disabled={isLoading}
        label={`Check status`}
        onClick={checkStatus}
      />

      <TaskStatus taskStatus={task?.status} />

      <Button disabled={!dataReady} label={`Load results`} onClick={loadData} />
    </>
  );
};

const FormIterator = ({ children, ...props }: any) => {
  return (
    <>
      <ScanButton label={props.label} />
      <AsyncResultLoader
        source={props.source}
        taskUrlFieldName={props.taskUrlFieldName}
      />

      <SimpleFormIterator {...props}>{children}</SimpleFormIterator>
    </>
  );
};

const StreamTranscriptInput = ({
  children,
  data: source,
  taskUrlFieldName,
  ...props
}: any) => {
  return (
    <ArrayInput source={source} {...props}>
      <FormIterator
        label={props.label}
        taskUrlFieldName={taskUrlFieldName}
        inline
      >
        {children}
      </FormIterator>
    </ArrayInput>
  );
};

export default StreamTranscriptInput;
