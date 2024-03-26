import {
  Edit,
  EditProps,
  ReferenceArrayInput,
  TextInput,
  TabbedForm,
} from "react-admin";

import StreamVideoClipsInput from "./StreamVideoClipsInput";
import StreamTranscriptInput from "./StreamTranscriptInput";
import StreamSilenceDetectionInput from "./StreamSilenceDetectionInput";
import DescriptionInput from "../../DescriptionInput";
import TitleInput from "../../TitleInput";

const StreamEdit = (props: EditProps) => (
  <Edit {...props}>
    <TabbedForm>
      <TabbedForm.Tab label="summary">
        <TitleInput source="title" />
        <DescriptionInput source="description" />
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
          source="transcription_segments"
          taskUrlFieldName="transcription_task_url"
        />
      </TabbedForm.Tab>

      <TabbedForm.Tab label="audio">
        <StreamSilenceDetectionInput
          source="silence_segments"
          taskUrlFieldName="silence_detection_task_url"
        />
      </TabbedForm.Tab>
    </TabbedForm>
  </Edit>
);

export default StreamEdit;
