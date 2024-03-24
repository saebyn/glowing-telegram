import {
  ArrayInput,
  DateInput,
  DeleteButton,
  Edit,
  SimpleForm,
  SimpleFormIterator,
  TextInput,
  TopToolbar,
} from "react-admin";
import { DurationInput } from "../../DurationInput";
import { ExportButton } from "../../OTIOExporter";

const EditActions = () => (
  <TopToolbar>
    <DeleteButton />
    <ExportButton />
  </TopToolbar>
);

const EpisodeEdit = () => (
  <Edit actions={<EditActions />}>
    <SimpleForm>
      <TextInput source="title" />
      <TextInput source="description" />

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
