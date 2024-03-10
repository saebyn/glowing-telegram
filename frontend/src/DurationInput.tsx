import React from "react";
import { CommonInputProps, useInput } from "react-admin";
import { TextFieldProps } from "@mui/material/TextField";
import { FormControl, FormHelperText } from "@mui/material";
import { Input } from "@mui/material";
import { Box } from "@mui/material";
import {
  Duration,
  parseISO8601Duration,
  toISO8601Duration,
} from "./isoDuration";

export type DurationInputProps = CommonInputProps &
  Omit<TextFieldProps, "helperText" | "label">;

const DurationField = (props: TextFieldProps) => {
  const { value, name, error, helperText, onChange } = props;

  const { hours, minutes, seconds, milliseconds } = parseISO8601Duration(
    value as string
  );

  const handleChange =
    (part: keyof Duration) => (event: React.ChangeEvent<HTMLInputElement>) => {
      const newValue = {
        hours,
        minutes,
        seconds,
        milliseconds,
        [part]: parseInt(event.target.value, 10),
      };

      if (onChange) {
        onChange({
          ...event,
          target: {
            ...event.target,
            value: toISO8601Duration(newValue),
            name: name || "",
          },
        });
      }
    };

  const commonProps = {
    error,
    required: props.required,
    variant: "outlined",
    type: "number",
    inputProps: {
      min: 0,
    },
    sx: {
      "& input": {
        textAlign: "right",
      },
    },
  } as const;

  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        gap: 1,
      }}
    >
      <FormHelperText error={error}>{helperText}</FormHelperText>

      <FormControl>
        <Input
          name={`${name}.hours`}
          onChange={handleChange("hours")}
          endAdornment="h"
          value={hours}
          {...commonProps}
        />
      </FormControl>
      <FormControl>
        <Input
          name={`${name}.minutes`}
          onChange={handleChange("minutes")}
          endAdornment="m"
          value={minutes}
          {...commonProps}
        />
      </FormControl>
      <FormControl>
        <Input
          name={`${name}.seconds`}
          onChange={handleChange("seconds")}
          endAdornment="s"
          value={seconds}
          {...commonProps}
        />
      </FormControl>
      <FormControl>
        <Input
          name={`${name}.milliseconds`}
          onChange={handleChange("milliseconds")}
          endAdornment="ms"
          value={milliseconds}
          {...commonProps}
        />
      </FormControl>
    </Box>
  );
};

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
      {...field}
      label={label}
      error={(isTouched || isSubmitted) && invalid}
      helperText={(isTouched || isSubmitted) && invalid ? error?.message : ""}
      required={isRequired}
      {...rest}
    />
  );
};
