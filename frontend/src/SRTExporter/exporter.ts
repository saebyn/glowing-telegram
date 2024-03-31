import { parseIntoSeconds } from "../isoDuration";
import { Episode, Stream } from "../types";

function exportSRT(episode: Episode, stream: Stream) {
  const transcriptSegments = (stream.transcription_segments || []).map(
    (segment) => ({
      start: parseIntoSeconds(segment.start),
      end: parseIntoSeconds(segment.end),
      text: segment.text,
    })
  );

  transcriptSegments.sort((a, b) => a.start - b.start);

  let srtString = "";

  let i = 1;

  for (const cut of episode.tracks) {
    const cutStart = parseIntoSeconds(cut.start);
    const cutEnd = parseIntoSeconds(cut.end);

    for (const transcriptSegment of transcriptSegments) {
      if (
        transcriptSegment.start >= cutStart &&
        transcriptSegment.end <= cutEnd
      ) {
        srtString += `${i}\n`;
        srtString += `${formatTime(
          transcriptSegment.start - cutStart
        )} --> ${formatTime(transcriptSegment.end - cutStart)}\n`;
        srtString += `${transcriptSegment.text}\n\n`;

        i++;
      }
    }
  }

  return srtString;
}

function formatTime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainingSeconds = seconds % 60;

  return `${pad(hours)}:${pad(minutes)}:${pad(remainingSeconds).replace(
    ".",
    ","
  )}0`;
}

function pad(num: number) {
  return num.toString().padStart(2, "0");
}

export default exportSRT;
