import simpleRestDataProvider from "ra-data-simple-rest";

const baseUrl = `${import.meta.env.VITE_API_URL || "http://localhost:3000"}`;

const baseDataProvider = simpleRestDataProvider(`${baseUrl}/records`);

interface TranscriptionAPIDetectInput {
  stream_id: string;
  uris: string[];
  track: number;
  language?: string;
  initial_prompt?: string;
}

interface SilenceDetectionAPIDetectInput {
  stream_id: string;
  uris: string[];
  track: number;

  noise?: number;
  duration?: number;
}

export const dataProvider = {
  ...baseDataProvider,

  // custom methods

  getStreamClips: async (prefix: string) => {
    const url = new URL(`${baseUrl}/stream_ingestion/find_files`);
    url.searchParams.append("prefix", prefix);

    return fetch(url).then((res) => res.json());
  },

  queueStreamTranscription: async ({
    stream_id: streamId,
    ...payload
  }: TranscriptionAPIDetectInput) => {
    return fetch(`${baseUrl}/transcription/detect`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    }).then(async (res) => {
      if (!res.ok) {
        throw new Error("Failed to queue transcription");
      }

      const taskUrl = res.headers.get("Location");

      if (!taskUrl) {
        throw new Error("Failed to queue transcription");
      }

      await fetch(`${baseUrl}/records/streams/${streamId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ transcription_task_url: taskUrl }),
      });

      return taskUrl;
    });
  },

  queueStreamSilenceDetection: async ({
    stream_id: streamId,
    ...payload
  }: SilenceDetectionAPIDetectInput) => {
    return fetch(`${baseUrl}/silence_detection/detect`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    }).then(async (res) => {
      if (!res.ok) {
        throw new Error("Failed to queue silence detection");
      }

      const taskUrl = res.headers.get("Location");

      if (!taskUrl) {
        throw new Error("Failed to queue silence detection");
      }

      await fetch(`${baseUrl}/records/streams/${streamId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ silence_detection_task_url: taskUrl }),
      });

      return taskUrl;
    });
  },

  getTask: async (taskUrl: string) => {
    return fetch(taskUrl).then((res) => res.json());
  },
};
