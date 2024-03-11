import {
  Box,
  FormHelperText,
  FormControl,
  Input,
  InputAdornment,
} from "@mui/material";
import { TextFieldProps } from "@mui/material/TextField";
import {
  Duration,
  parseISO8601Duration,
  toISO8601Duration,
} from "../../isoDuration";

export type DurationFieldProps = TextFieldProps;

const DurationField = (props: DurationFieldProps) => {
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
      <FormHelperText id={`${name}-helper-text`} error={error}>
        {helperText}
      </FormHelperText>

      <FormControl variant="standard">
        <Input
          name={`${name}.hours`}
          onChange={handleChange("hours")}
          endAdornment={<InputAdornment position="end">h</InputAdornment>}
          aria-describedby={`${name}-hours-helper-text`}
          value={hours}
          {...commonProps}
          inputProps={{
            "aria-label": "hours",
          }}
        />
        <FormHelperText id={`${name}-hours-helper-text`}>Hours</FormHelperText>
      </FormControl>
      <FormControl variant="standard">
        <Input
          name={`${name}.minutes`}
          onChange={handleChange("minutes")}
          endAdornment={<InputAdornment position="end">m</InputAdornment>}
          aria-describedby={`${name}-minutes-helper-text`}
          value={minutes}
          {...commonProps}
          inputProps={{
            "aria-label": "minutes",
          }}
        />
        <FormHelperText id={`${name}-minutes-helper-text`}>
          Minutes
        </FormHelperText>
      </FormControl>
      <FormControl variant="standard">
        <Input
          name={`${name}.seconds`}
          onChange={handleChange("seconds")}
          endAdornment={<InputAdornment position="end">s</InputAdornment>}
          aria-describedby={`${name}-seconds-helper-text`}
          value={seconds}
          {...commonProps}
          inputProps={{
            "aria-label": "seconds",
          }}
        />
        <FormHelperText id={`${name}-seconds-helper-text`}>
          Seconds
        </FormHelperText>
      </FormControl>
      <FormControl variant="standard">
        <Input
          name={`${name}.milliseconds`}
          onChange={handleChange("milliseconds")}
          endAdornment={<InputAdornment position="end">ms</InputAdornment>}
          aria-describedby={`${name}-milliseconds-helper-text`}
          value={milliseconds}
          {...commonProps}
          inputProps={{
            "aria-label": "milliseconds",
          }}
        />
        <FormHelperText id={`${name}-milliseconds-helper-text`}>
          Milliseconds
        </FormHelperText>
      </FormControl>
    </Box>
  );
};

export default DurationField;
