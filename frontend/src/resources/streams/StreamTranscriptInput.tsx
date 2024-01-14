import {
  Button,
  ArrayInput,
  SimpleFormIterator,
  useRecordContext,
  useDataProvider,
  useRefresh,
} from "react-admin";
import { useMutation } from "react-query";

const ScanButton = ({ label }: { label: string }) => {
  const record = useRecordContext();
  const refresh = useRefresh();
  const dataProvider = useDataProvider();

  const { mutate, isLoading } = useMutation<string | null>(() =>
    dataProvider.queueStreamTranscription({
      uris: ["file:/2024-01-07 08-27-37.mkv"],
      track: 2,
      initial_prompt:
        "Title: rust APIs + react-admin project continues | Chill Sunday Morning Coding\nDescription: Software and Game Development on twitch.tv/saebyn\n",
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

const FormIterator = ({ children, ...props }: any) => {
  return (
    <>
      <ScanButton label={props.label} />

      <SimpleFormIterator {...props}>{children}</SimpleFormIterator>
    </>
  );
};

const StreamTranscriptInput = ({ children, data: source, ...props }: any) => {
  return (
    <ArrayInput source={source} {...props}>
      <FormIterator label={props.label} inline>
        {children}
      </FormIterator>
    </ArrayInput>
  );
};

export default StreamTranscriptInput;
