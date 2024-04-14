import {
  ArrayInput,
  DeleteButton,
  Edit,
  ReferenceInput,
  SimpleForm,
  SimpleFormIterator,
  SelectInput,
  TopToolbar,
} from "react-admin";
import { DurationInput } from "../../DurationInput";
import { ExportButton as OTIOExportButton } from "../../OTIOExporter";
import { ExportButton as SRTExportButton } from "../../SRTExporter";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";
import MediaPickerInput from "../../MediaPickerInput";

const EditActions = () => (
  <TopToolbar>
    <DeleteButton />
    <OTIOExportButton />
    <SRTExportButton />
  </TopToolbar>
);

const EpisodeEdit = () => (
  <Edit actions={<EditActions />}>
    <SimpleForm>
      <TitleInput source="title" />
      <DescriptionInput source="description" />

      {/* TODO add file url entry that opens a dialog that pulls the list of files from /api/stream_ingestion/find_rendered_episode_files */}
      <MediaPickerInput source="render_uri" type="render" />
      {/* 
      The plan:
      - Create the MediaPicker component implements the react-admin Input interface, uses the API to fetch the list of files, and uses a new MediaPicker component to display the list of files and select one

      */}

      <ArrayInput source="tracks">
        <SimpleFormIterator>
          <DurationInput source="start" />
          <DurationInput source="end" />
        </SimpleFormIterator>
      </ArrayInput>

      <ReferenceInput source="stream_id" reference="streams">
        <SelectInput optionText="title" />
      </ReferenceInput>
    </SimpleForm>
  </Edit>
);

export default EpisodeEdit;
