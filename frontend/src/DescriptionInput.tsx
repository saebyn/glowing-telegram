/**
 * DescriptionInput component
 *
 * This component is used to input the description of a record,
 * and is used in the Edit and Create views. It extends the
 * TextInput component from react-admin.
 */

import * as React from "react";
import { maxLength, TextInput } from "react-admin";

// Derive the Props type from the TextInput component
type Props = React.ComponentProps<typeof TextInput>;

const inputProps = {
  placeholder: "Enter the description here",
};

const muiInputProps = {};

const descriptionValidators = [maxLength(5000)];

const DescriptionInput = (props: Props) => (
  <TextInput
    {...props}
    label="Description"
    inputProps={inputProps}
    InputProps={muiInputProps}
    fullWidth
    multiline
    minRows={4}
    maxRows={16}
    validate={descriptionValidators}
    resettable
  />
);

export default DescriptionInput;
