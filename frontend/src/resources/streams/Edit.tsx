import {
  Edit,
  EditProps,
  ReferenceArrayInput,
  TextInput,
  TabbedForm,
  SelectInput,
  DateTimeInput,
} from "react-admin";

import StreamVideoClipsInput from "./StreamVideoClipsInput";
import StreamTranscriptInput from "./StreamTranscriptInput";
import StreamSilenceDetectionInput from "./StreamSilenceDetectionInput";
import DescriptionInput from "../../DescriptionInput";
import TitleInput from "../../TitleInput";
import { DurationInput } from "../../DurationInput";

const StreamEdit = (props: EditProps) => (
  <Edit {...props}>
    <TabbedForm>
      <TabbedForm.Tab label="summary">
        <TitleInput source="title" required />
        <DescriptionInput source="description" />

        <TextInput source="thumbnail" fullWidth />

        <SelectInput
          source="stream_platform"
          choices={[
            { id: "twitch", name: "Twitch" },
            { id: "youtube", name: "YouTube" },
          ]}
          required
          defaultValue="twitch"
        />
        <TextInput source="stream_id" />

        <DateTimeInput source="stream_date" required />

        <DurationInput source="duration" />

        <TextInput
          source="prefix"
          required
          helperText="The prefix is used to identify related video clips for this stream. It's typically in the format YYYY-MM-DD."
          inputProps={{ pattern: "[0-9]{4}-[0-9]{2}-[0-9]{2}" }}
        />

        <ReferenceArrayInput
          source="topic_ids"
          reference="topics"
          label="Topics"
        />
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
