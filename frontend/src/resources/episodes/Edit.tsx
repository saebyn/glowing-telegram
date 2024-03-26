import {
  ArrayInput,
  DateInput,
  DeleteButton,
  Edit,
  SimpleForm,
  SimpleFormIterator,
  TopToolbar,
} from "react-admin";
import { DurationInput } from "../../DurationInput";
import { ExportButton } from "../../OTIOExporter";
import TitleInput from "../../TitleInput";
import DescriptionInput from "../../DescriptionInput";

const EditActions = () => (
  <TopToolbar>
    <DeleteButton />
    <ExportButton />
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

      <DateInput source="updated_at" />
      <DateInput source="created_at" />
    </SimpleForm>
  </Edit>
);

export default EpisodeEdit;
