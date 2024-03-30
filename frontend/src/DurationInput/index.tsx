import { CommonInputProps, useInput } from "react-admin";
import { TextFieldProps } from "@mui/material/TextField";
import DurationField from "./mui/DurationField";

export type DurationInputProps = CommonInputProps &
  Omit<TextFieldProps, "helperText" | "label">;

export const DurationInput = (props: DurationInputProps) => {
  const { onChange, onBlur, label, ...rest } = props;
  const {
    field,
    fieldState: { isTouched, invalid, error },
    formState: { isSubmitted },
    isRequired,
  } = useInput({
    onChange,
    onBlur,
    ...rest,
  });

  return (
    <DurationField
      onChange={field.onChange}
      onBlur={field.onBlur}
      value={field.value}
      name={field.name}
      disabled={field.disabled}
      label={label}
      error={(isTouched || isSubmitted) && invalid}
      helperText={(isTouched || isSubmitted) && invalid ? error?.message : ""}
      required={isRequired}
      {...rest}
    />
  );
};
