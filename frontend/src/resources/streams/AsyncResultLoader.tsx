import { useState } from "react";
import {
  Button,
  useRecordContext,
  useDataProvider,
  useNotify,
} from "react-admin";
import { useFormContext } from "react-hook-form";
import { TaskStatus } from "./StreamTranscriptInput";

interface Task<T> {
  status: "Queued" | "Processing" | "Complete" | "Failed";
  id: string;
  data: T[];
}
export default function AsyncResultLoader<T>({
  source,
  taskUrlFieldName,
}: {
  source: string;
  taskUrlFieldName: string;
}) {
  const record = useRecordContext();
  const dataProvider = useDataProvider();
  const formContext = useFormContext();
  const notify = useNotify();

  const [task, setTask] = useState<Task<T> | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  if (!record || !record[taskUrlFieldName]) {
    return null;
  }

  const dataReady = task?.status === "Complete";

  const checkStatus = async () => {
    setIsLoading(true);

    try {
      const taskData = await dataProvider.getTask(record[taskUrlFieldName]);
      setTask(taskData);
    } catch (e) {
      notify(`Failed to get task: ${e}`, { type: "error" });
    }

    setIsLoading(false);
  };

  const loadData = () => {
    const values: T[] = task?.data || [];

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
}
