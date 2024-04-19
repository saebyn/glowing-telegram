import { Button, useDataProvider, useListContext } from "react-admin";
import { useState } from "react";

import { YoutubeUploadTaskPayload } from "../../types";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import MuiButton from "@mui/material/Button";

import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";

const UploadEpisodeToYoutubeButton = () => {
  const [open, setOpen] = useState(false);
  const [episodes, setEpisodes] = useState<YoutubeUploadTaskPayload[]>([]);
  const { selectedIds } = useListContext();
  const dataProvider = useDataProvider();

  const handleUpload = async () => {
    await Promise.all(
      episodes
        .filter((episode: any) => episode.render_uri)
        .map(dataProvider.uploadEpisodeToYoutube)
    );

    setOpen(false);
  };

  const handleOpen = async () => {
    const { data } = await dataProvider.getMany("episodes", {
      ids: selectedIds,
    });

    setEpisodes(
      data.map((episode: any) => ({
        title: episode.title,
        description: episode.description,
        render_uri: episode.render_uri,
        category: 20,
        tags: [],
        notify_subscribers: false,
        task_title: `Upload ${episode.title} to Youtube`,
      }))
    );
    setOpen(true);
  };
  const handleClose = () => setOpen(false);

  return (
    <>
      <Dialog open={open} onClose={handleClose}>
        <DialogTitle>Upload to Youtube</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Are you sure you want to upload the selected episodes to Youtube?
          </DialogContentText>

          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Title</TableCell>
                <TableCell>Can Upload?</TableCell>
                <TableCell>Category</TableCell>
                <TableCell>Tags</TableCell>
                <TableCell>Notify subscribers</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {episodes.map((episode) => (
                <TableRow
                  key={episode.title}
                  color={episode.render_uri ? "success" : "error"}
                >
                  <TableCell>{episode.title}</TableCell>
                  <TableCell>{episode.render_uri ? "Yes" : "No"}</TableCell>
                  <TableCell>{episode.category}</TableCell>
                  <TableCell>{episode.tags.join(", ")}</TableCell>
                  <TableCell>
                    {episode.notify_subscribers ? "Yes" : "No"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </DialogContent>
        <DialogActions>
          <MuiButton onClick={handleClose}>Cancel</MuiButton>
          <MuiButton onClick={handleUpload} color="primary">
            Upload
          </MuiButton>
        </DialogActions>
      </Dialog>

      <Button label="Upload to Youtube" onClick={handleOpen} />
    </>
  );
};

export default UploadEpisodeToYoutubeButton;
