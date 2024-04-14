import { FC, useState } from "react";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import Radio from "@mui/material/Radio";
import InputLabel from "@mui/material/InputLabel";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@mui/material";

interface Metadata {
  filename: string;
  size: number;
}

export interface MediaEntry {
  uri: string;
  metadata: Metadata;
}

interface MediaPickerProps {
  entries: MediaEntry[];
  onChoose: (_entry: MediaEntry) => void;
  value: string | null;
  label?: string;
}

const MediaPicker: FC<MediaPickerProps> = ({
  entries,
  onChoose,
  value,
  label,
}) => {
  const [choosing, setChoosing] = useState(false);

  const defaultEntry = entries.find((entry) => entry.uri === value);

  const [entry, setEntry] = useState<MediaEntry | undefined>(defaultEntry);

  const handleChoose = () => {
    if (!entry) {
      return;
    }
    setChoosing(false);
    onChoose(entry);
  };

  const handleCancel = () => {
    setEntry(defaultEntry);
    setChoosing(false);
  };

  return (
    <>
      <InputLabel>{label || "Media file"}</InputLabel>
      <TextField
        label={value ? "" : "No file selected"}
        value={entry?.metadata.filename || ""}
        disabled
      />
      <Button
        variant="contained"
        color="primary"
        onClick={() => setChoosing(true)}
      >
        Browse...
      </Button>

      <Dialog open={choosing} onClose={() => setChoosing(false)}>
        <DialogTitle>Select a media file</DialogTitle>
        <DialogContent>
          <List>
            {entries.map((thisEntry) => (
              <ListItem key={thisEntry.uri}>
                <ListItemButton
                  onClick={() => setEntry(thisEntry)}
                  selected={entry?.uri === thisEntry.uri}
                >
                  <ListItemIcon>
                    <Radio
                      edge="start"
                      checked={entry?.uri === thisEntry.uri}
                      tabIndex={-1}
                      disableRipple
                    />
                  </ListItemIcon>
                  <ListItemText primary={thisEntry.metadata.filename} />
                </ListItemButton>
              </ListItem>
            ))}
          </List>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => handleCancel()}>Cancel</Button>
          <Button
            variant="contained"
            color="primary"
            disabled={!entry}
            onClick={() => handleChoose()}
          >
            Choose
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};

export default MediaPicker;
