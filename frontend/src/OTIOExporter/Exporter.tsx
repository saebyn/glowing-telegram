import {
  Button,
  useDataProvider,
  useGetOne,
  useListContext,
  useRecordContext,
} from "react-admin";
import DownloadIcon from "@mui/icons-material/Download";

import exporter from "./export";
import { parseIntoSeconds } from "../isoDuration";

function promptDownload(episode: any, stream: any) {
  const videoClips = stream.video_clips.map((clip: any) => ({
    uri: clip.uri.replace("file:local:", ""),
    duration: parseIntoSeconds(clip.duration),
    start: clip.start,
  }));

  videoClips.sort((a: any, b: any) => a.start - b.start);

  // take the episode data and use the OTIOExporter to genrate the OTIO string
  // then create a blob object and create a download link
  // then click the link to download the file
  const otioString = exporter(
    {
      title: episode.title,
      description: episode.description,
      tracks: episode.tracks.map((track: { start: string; end: string }) => ({
        start: track.start,
        end: track.end,
      })),
    },
    {
      video_clips: videoClips,
    }
  );

  const blob = new Blob([otioString], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;

  // set the filename to the episode name
  a.download = `${episode.title}.otio`;
  a.click();
}

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

    promptDownload(episode, stream);
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

export const BulkExportButton = () => {
  const { selectedIds } = useListContext();
  const dataProvider = useDataProvider();

  const handleExport = async () => {
    for (const id of selectedIds) {
      const { data: episode } = await dataProvider.getOne("episodes", {
        id,
      });

      if (!episode.tracks || episode.tracks.length === 0) {
        alert("Episode has no cuts to export.");
        return;
      }

      const { data: stream } = await dataProvider.getOne("streams", {
        id: episode.stream_id,
      });

      if (!stream.video_clips || stream.video_clips.length === 0) {
        alert("Stream has no video clips to export.");
        return;
      }

      promptDownload(episode, stream);
    }
  };

  return <Button label="Bulk Export OTIO" onClick={handleExport} />;
};
