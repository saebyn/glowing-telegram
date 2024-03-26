import { Button, useGetOne, useRecordContext } from "react-admin";
import DownloadIcon from "@mui/icons-material/Download";

import exporter from "./export";
import { parseIntoSeconds } from "../isoDuration";

export const ExportButton = () => {
  const episode = useRecordContext();
  const {
    data: stream,
    isLoading,
    error,
    refetch,
  } = useGetOne(
    "streams",
    { id: episode?.stream_id },
    {
      enabled: !!episode,
    }
  );

  if (!episode) {
    return null;
  }

  if (error) {
    // show an error message and a retry button
    return (
      <div>
        <p>There was an error loading the stream data.</p>
        <Button onClick={() => refetch()} label="Retry" />
      </div>
    );
  }

  const handleExport = () => {
    if (!episode || !stream) return;
    if (isLoading) return;
    if (error) return;
    if (!episode.tracks || episode.tracks.length === 0) {
      alert("Episode has no cuts to export.");
      return;
    }
    if (!stream.video_clips || stream.video_clips.length === 0) {
      alert("Stream has no video clips to export.");
      return;
    }

    // take the episode data and use the OTIOExporter to genrate the OTIO string
    // then create a blob object and create a download link
    // then click the link to download the file
    const otioString = exporter(
      {
        name: episode.name,
        description: episode.description,
        cuts: episode.tracks.map((track: { start: string; end: string }) => ({
          start: parseIntoSeconds(track.start),
          end: parseIntoSeconds(track.end),
        })),
      },
      {
        videoClips: stream.video_clips.map((clip: any) => ({
          uri: clip.uri.replace("file:local:", ""),
          duration: parseIntoSeconds(clip.duration),
        })),
      }
    );

    const blob = new Blob([otioString], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;

    // set the filename to the episode name
    a.download = `${episode.title}.otio`;
    a.click();
  };

  return (
    <Button
      label="Export OTIO"
      onClick={handleExport}
      startIcon={<DownloadIcon />}
      disabled={isLoading}
    />
  );
};
