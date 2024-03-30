import {
  Create,
  SimpleForm,
  CreateProps,
  TextInput,
  SelectInput,
  DateTimeInput,
} from "react-admin";
import DescriptionInput from "../../DescriptionInput";
import TitleInput from "../../TitleInput";
import { DurationInput } from "../../DurationInput";

const StreamCreate = (props: CreateProps) => (
  <Create {...props} title="Create a Stream">
    <SimpleForm>
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
    </SimpleForm>
  </Create>
);

export default StreamCreate;
