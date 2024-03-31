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
