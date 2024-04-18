import {
  ArrayInput,
  DeleteButton,
  ReferenceInput,
  SimpleForm,
  SimpleFormIterator,
  SelectInput,
  TopToolbar,
  PrevNextButtons,
} from "react-admin";
import { DurationInput } from "../../DurationInput";
import { ExportButton as OTIOExportButton } from "../../OTIOExporter";
import { ExportButton as SRTExportButton } from "../../SRTExporter";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";
import MediaPickerInput from "../../MediaPickerInput";
import Edit from "../../Edit";

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
    <SimpleForm>
      <TitleInput source="title" />
      <DescriptionInput source="description" />

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
    </SimpleForm>
  </Edit>
);

export default EpisodeEdit;
