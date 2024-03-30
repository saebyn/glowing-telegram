import {
  Datagrid,
  DateField,
  List,
  TextField,
  ListProps,
  CloneButton,
  TextInput,
  Filter,
  useListContext,
} from "react-admin";
import { DateCalendar } from "@mui/x-date-pickers/DateCalendar";
import { PickersDay } from "@mui/x-date-pickers/PickersDay";
import ThumbnailField from "../../ThumbnailField";
import { Badge } from "@mui/material";

// TODO add q to the crud api
const StreamsFilter = (props: any) => (
  <Filter {...props}>
    <TextInput label="Search" source="q" alwaysOn />
  </Filter>
);

function getDateKey(date: Date): string {
  return `${date.getFullYear()}-${(date.getMonth() + 1)
    .toString()
    .padStart(2, "0")}-${date.getDate().toString().padStart(2, "0")}`;
}

const StreamDay = ({
  days,
  day,

  ...props
}: any) => {
  const dayStr = getDateKey(day);
  const count = days[dayStr] || 0;

  return (
    <Badge
      key={day.toString()}
      overlap="circular"
      badgeContent={count}
      variant="dot"
      color="primary"
    >
      <PickersDay {...props} day={day} />
    </Badge>
  );
};

const calendarStyle = {
  display: "flex",
  minWidth: 300,
  flexDirection: "column",
  alignItems: "center",
  "& .MuiPickersCalendar-week": {
    display: "flex",
    justifyContent: "center",
  },
  "& .MuiPickersCalendar-transitionContainer": {
    width: "100%",
  },
};

const CalendarView = () => {
  const list = useListContext();

  const days: Record<string, number> = {};

  if (list.data) {
    list.data.forEach((stream: any) => {
      if (!stream || !stream.stream_date) {
        return;
      }
      const date = new Date(stream.stream_date);
      const key = getDateKey(date);
      days[key] = (days[key] || 0) + 1;
    });
  }

  return (
    <DateCalendar
      sx={calendarStyle}
      showDaysOutsideCurrentMonth
      slots={{
        day: StreamDay,
      }}
      slotProps={{
        day: {
          days,
        } as any,
      }}
    />
  );
};

const StreamList = (props: ListProps) => (
  <List {...props} filters={<StreamsFilter />} aside={<CalendarView />}>
    <Datagrid rowClick="edit">
      <DateField source="stream_date" />
      <TextField source="prefix" />
      <TextField source="title" />
      <ThumbnailField source="thumbnail" width={100} height={100} />
      <CloneButton />
    </Datagrid>
  </List>
);

export default StreamList;
