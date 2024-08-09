import {
  ArrayInput,
  DeleteButton,
  ReferenceInput,
  SimpleForm,
  SimpleFormIterator,
  SelectInput,
  TopToolbar,
  TextInput,
  NumberInput,
  PrevNextButtons,
  useRecordContext,
  useGetOne,
  BooleanInput,
} from "react-admin";

import { useFormContext } from "react-hook-form";

import { DurationInput } from "../../DurationInput";
import { ExportButton as OTIOExportButton } from "../../OTIOExporter";
import { ExportButton as SRTExportButton } from "../../SRTExporter";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";
import MediaPickerInput from "../../MediaPickerInput";
import Edit from "../../Edit";
import ChatButton from "../../ChatButton";
import { Episode, TranscriptSegment } from "../../types";
import { parseIntoSeconds } from "../../isoDuration";
import YouTubeCategoryInput from "../../YouTubeCategoryInput";

const EditActions = () => (
  <TopToolbar>
    <PrevNextButtons />
    <DeleteButton />
    <OTIOExportButton />
    <SRTExportButton />
  </TopToolbar>
);

const EpisodeEdit = () => (
  <Edit actions={<EditActions />}>
    <SimpleForm mode="onBlur" reValidateMode="onBlur">
      <TitleInput source="title" />

      <ReferenceInput source="series_id" reference="series">
        <SelectInput
          optionText={(record) =>
            `${record.title} (${record.max_episode_order_index})`
          }
        />
      </ReferenceInput>

      <NumberInput source="order_index" />

      <BooleanInput source="is_published" />

      <DescriptionInput source="description" />

      <EpisodeDescriptionChatButton />

      <MediaPickerInput source="render_uri" type="render" />

      <ArrayInput source="tracks">
        <SimpleFormIterator>
          <DurationInput source="start" />
          <DurationInput source="end" />
        </SimpleFormIterator>
      </ArrayInput>

      <ReferenceInput source="stream_id" reference="streams">
        <SelectInput optionText="title" />
      </ReferenceInput>

      <BooleanInput source="notify_subscribers" />
      <YouTubeCategoryInput source="category" />
      <ArrayInput source="tags">
        <SimpleFormIterator>
          <TextInput source="" />
        </SimpleFormIterator>
      </ArrayInput>
    </SimpleForm>
  </Edit>
);

const EpisodeDescriptionChatButton = () => {
  const { setValue } = useFormContext();
  const record = useRecordContext<Episode>();

  const { data: stream } = useGetOne(
    "streams",
    {
      id: record?.stream_id,
    },
    {
      enabled: !!record?.stream_id,
    },
  );

  if (!record) {
    return null;
  }

  if (!stream) {
    return null;
  }

  const job = `I summarize the provided video transcript into a title and description of the video to optimize for finding this video on youtube.
  My response is a well-formed JSON object that includes the title and description. It should look like this:

  {
    "title": "Title of the video",
    "description": "Description of the video content \n\n On as many lines as needed."
  }
  `;

  const context = `
    I need help summarizing the video transcript into a title and description for the video. I would prefer the text to be written in the first person. I would like the title to be a maximum of 100 characters and the description to be a maximum of 5000 characters. I would like the description to be broken up into paragraphs and formatted for readability.

    The tentative title of the video is "${record.title}".
    The stream was recorded on ${stream.stream_date}.

    The base description is:
    "${record.description}"


    Here is the transcript:
  `;

  const transcriptionSegments = stream.transcription_segments;

  if (!transcriptionSegments) {
    return null;
  }

  let episodeStart: null | number = null;

  const transcript = transcriptionSegments
    .filter((segment: TranscriptSegment) =>
      transcriptSegmentOverlaps(segment, record),
    )
    .map((segment: TranscriptSegment) => {
      if (episodeStart === null) {
        episodeStart = parseIntoSeconds(segment.start);
      }

      const start = Math.round(parseIntoSeconds(segment.start) - episodeStart);

      return `${start}s: ${segment.text}`;
    })
    .join("\n");

  const handleChange = (content: string) => {
    const json = JSON.parse(content);

    setValue("title", json.title);
    setValue("description", json.description);
  };

  return (
    <ChatButton
      job={job}
      transcript={transcript}
      context={context}
      onChange={handleChange}
    />
  );
};

function transcriptSegmentOverlaps(
  segment: TranscriptSegment,
  record: Episode,
): boolean {
  if (!record.tracks || record.tracks.length === 0) {
    return false;
  }

  const startTranscript = parseIntoSeconds(segment.start);
  const endTranscript = parseIntoSeconds(segment.end);

  for (const { start, end } of record.tracks) {
    const startCut = parseIntoSeconds(start);
    const endCut = parseIntoSeconds(end);

    if (startTranscript >= startCut && startTranscript <= endCut) {
      return true;
    }

    if (endTranscript >= startCut && endTranscript <= endCut) {
      return true;
    }
  }

  return false;
}

function formatYoutubeChapterTimestampsFromSeconds(seconds: number): string {
  const hours = String(Math.floor(seconds / 3600)).padStart(2, "0");
  const minutes = String(Math.floor((seconds % 3600) / 60)).padStart(2, "0");
  const remainingSeconds = String(Math.floor(seconds % 60)).padStart(2, "0");

  return `${hours}:${minutes}:${remainingSeconds}`;
}

export default EpisodeEdit;
