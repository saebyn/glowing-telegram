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
  Button,
  useDataProvider,
  LoadingIndicator,
} from "react-admin";
import { DateCalendar } from "@mui/x-date-pickers/DateCalendar";
import { PickersDay } from "@mui/x-date-pickers/PickersDay";
import ThumbnailField from "../../ThumbnailField";
import Badge from "@mui/material/Badge";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogActions from "@mui/material/DialogActions";
import MuiTextField from "@mui/material/TextField";
import MuiButton from "@mui/material/Button";
import React, { useState } from "react";

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

const BulkSilenceDetectionButton = () => {
  const [track, setTrack] = useState(2);
  const [duration, setDuration] = useState(30);
  const [open, setOpen] = useState(false);
  const [processing, setProcessing] = useState(false);
  const { selectedIds } = useListContext();
  const dataProvider = useDataProvider();

  const onSilenceDetection = async () => {
    setProcessing(true);
    await Promise.all(
      selectedIds.map(async (streamId) => {
        const { data: stream } = await dataProvider.getOne("streams", {
          id: streamId,
        });
        await dataProvider.queueStreamSilenceDetection({
          uris: stream.video_clips.map((clip: any) => clip.uri),
          track,
          duration,
          stream_id: stream.id,
        });
      })
    );

    setProcessing(false);
    setOpen(false);
  };

  return (
    <>
      <Dialog open={open} onClose={() => setOpen(false)}>
        <DialogTitle>Silence Detection</DialogTitle>

        {processing ? (
          <DialogContent>
            <LoadingIndicator />
          </DialogContent>
        ) : (
          <DialogContent>
            <MuiTextField
              label="Track"
              type="number"
              value={track}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                setTrack(parseInt(e.target.value, 10))
              }
            />
            <MuiTextField
              label="Duration"
              type="number"
              value={duration}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                setDuration(parseInt(e.target.value, 10))
              }
            />
          </DialogContent>
        )}

        <DialogActions>
          <MuiButton disabled={processing} onClick={() => setOpen(false)}>
            Cancel
          </MuiButton>
          <MuiButton
            disabled={processing}
            onClick={onSilenceDetection}
            color="primary"
          >
            Start Silence Detection
          </MuiButton>
        </DialogActions>
      </Dialog>

      <Button label="Silence Detection" onClick={() => setOpen(true)} />
    </>
  );
};

const StreamBulkActions = () => (
  <>
    <BulkSilenceDetectionButton />
  </>
);

const StreamList = (props: ListProps) => (
  <List {...props} filters={<StreamsFilter />} aside={<CalendarView />}>
    <Datagrid rowClick="edit" bulkActionButtons={<StreamBulkActions />}>
      <DateField source="stream_date" />
      <TextField source="prefix" />
      <TextField source="title" />
      <ThumbnailField source="thumbnail" width={100} height={100} />
      <CloneButton />
    </Datagrid>
  </List>
);

export default StreamList;
