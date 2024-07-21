import React, { useState } from "react";
import {
  Datagrid,
  DateField,
  List,
  TextField,
  NumberField,
  ListProps,
  CloneButton,
  useListContext,
  BooleanField,
  Button,
  useDataProvider,
  LoadingIndicator,
  SearchInput,
  NullableBooleanInput,
  ReferenceInput,
  SelectInput,
  DateInput,
  ReferenceField,
} from "react-admin";
import { DateCalendar } from "@mui/x-date-pickers/DateCalendar";
import { PickersDay } from "@mui/x-date-pickers/PickersDay";
import Badge from "@mui/material/Badge";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogActions from "@mui/material/DialogActions";
import MuiTextField from "@mui/material/TextField";
import MuiButton from "@mui/material/Button";
import { FindFilesResponse } from "../../types";

/* eslint-disable react/jsx-key */
const streamsFilter = [
  <SearchInput source="q" alwaysOn />,

  <NullableBooleanInput source="has_transcription" label="Transcription" />,
  <NullableBooleanInput
    source="has_silence_detection"
    label="Silence Detection"
  />,
  <NullableBooleanInput source="has_video_clips" label="Video Clips" />,
  <NullableBooleanInput source="has_episodes" label="Episodes" />,

  <DateInput source="stream_date__gte" label="Stream Date After" />,

  <ReferenceInput source="series_id" reference="series">
    <SelectInput optionText="title" />
  </ReferenceInput>,
];

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
          task_title: `Silence Detection for ${stream.title}`,
          uris: stream.video_clips.map((clip: any) => clip.uri),
          track,
          duration,
          stream_id: stream.id,
        });
      }),
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

const BulkScanForClipsButton = () => {
  const { selectedIds, refetch, onSelect } = useListContext();
  const dataProvider = useDataProvider();

  const onScanForClips = async () => {
    await Promise.all(
      selectedIds.map(async (streamId) => {
        const { data: stream } = await dataProvider.getOne("streams", {
          id: streamId,
        });
        const clips: FindFilesResponse = await dataProvider.getStreamClips(
          stream.prefix,
        );

        await dataProvider.update("streams", {
          id: streamId,
          previousData: stream,
          data: {
            video_clips: clips.entries.map((entry) => ({
              title: entry.metadata.filename,
              uri: entry.uri,
              duration: entry.metadata.duration,
              start_time: entry.metadata.start_time,
              audio_bitrate: entry.metadata.audio_bitrate,
              audio_track_count: entry.metadata.audio_track_count,
              content_type: entry.metadata.content_type,
              filename: entry.metadata.filename,
              frame_rate: entry.metadata.frame_rate,
              height: entry.metadata.height,
              width: entry.metadata.width,
              video_bitrate: entry.metadata.video_bitrate,
              size: entry.metadata.size,
              last_modified: entry.metadata.last_modified,
            })),
          },
        });

        onSelect([]);

        await refetch();
      }),
    );
  };

  return <Button label="Scan for Clips" onClick={onScanForClips} />;
};

const StreamBulkActions = () => (
  <>
    <BulkSilenceDetectionButton />
    <BulkScanForClipsButton />
  </>
);

const StreamList = (props: ListProps) => (
  <List {...props} filters={streamsFilter} aside={<CalendarView />}>
    <Datagrid rowClick="edit" bulkActionButtons={<StreamBulkActions />}>
      <DateField source="stream_date" />
      <TextField source="title" />
      <ReferenceField source="series_id" reference="series">
        <TextField source="title" />
      </ReferenceField>
      <NumberField source="video_clip_count" />
      <BooleanField source="has_transcription" />
      <BooleanField source="has_silence_detection" />
      <BooleanField source="has_episodes" />
      <CloneButton />
    </Datagrid>
  </List>
);

export default StreamList;
