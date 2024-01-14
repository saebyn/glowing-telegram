import {
  Edit,
  EditProps,
  ReferenceArrayInput,
  TextInput,
  TabbedForm,
} from "react-admin";

import StreamVideoClipsInput from "./StreamVideoClipsInput";
import StreamTranscriptInput from "./StreamTranscriptInput";

const StreamEdit = (props: EditProps) => (
  <Edit {...props}>
    <TabbedForm>
      <TabbedForm.Tab label="summary">
        <TextInput source="title" />
        <TextInput multiline={true} source="description" />
        <TextInput source="prefix" />
        <TextInput source="speech_audio_track" />
        <TextInput source="thumbnail" />
        <ReferenceArrayInput source="topic_ids" reference="topics">
          <TextInput source="id" />
        </ReferenceArrayInput>
      </TabbedForm.Tab>

      <TabbedForm.Tab label="video clips">
        <StreamVideoClipsInput source="video_clips" />
      </TabbedForm.Tab>

      <TabbedForm.Tab label="transcript">
        <StreamTranscriptInput
          data="transcription_segments"
          task="transcription_task_url"
          label="Transcription"
        >
          <TextInput source="start" />
          <TextInput source="end" />
          <TextInput source="text" />
        </StreamTranscriptInput>
      </TabbedForm.Tab>
    </TabbedForm>
  </Edit>
);

export default StreamEdit;
