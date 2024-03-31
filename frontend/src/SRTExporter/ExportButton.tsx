import DownloadIcon from "@mui/icons-material/Download";

import { useRecordContext, useGetOne, Button } from "react-admin";

import exportSRT from "./exporter";
import { Episode } from "../types";

const ExportButton = () => {
  const episode = useRecordContext<Episode>();
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
    if (
      !stream.transcription_segments ||
      stream.transcription_segments.length === 0
    ) {
      alert("Stream has no transcript to export.");
      return;
    }

    const srtString = exportSRT(episode, stream);

    const blob = new Blob([srtString], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;

    // set the filename to the episode name
    a.download = `${episode.title}.srt`;
    a.click();
  };

  return (
    <Button
      label="Export Captions as SRT"
      onClick={handleExport}
      startIcon={<DownloadIcon />}
      disabled={isLoading}
    />
  );
};

export default ExportButton;
