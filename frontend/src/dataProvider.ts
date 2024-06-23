import simpleRestDataProvider from "ra-data-simple-rest";
import { DataProvider, GetListParams, combineDataProviders } from "react-admin";
import { ChatMessage, YoutubeUploadTaskPayload } from "./types";
import GTWebSocket, { Callback as WebSocketCallback } from "./websocket";

const baseUrl = `${import.meta.env.VITE_API_URL || "http://localhost:3000"}`;

const crudDataProvider = simpleRestDataProvider(`${baseUrl}/records`);

const twitchVideosDataProvider = {
  cursorPage: 1,
  cursor: "",

  async getList(
    this: { cursorPage: number; cursor: string },
    _resource: string,
    params: GetListParams
  ) {
    const page = params.pagination?.page || 1;
    let cursor = "";

    if (page === this.cursorPage) {
      cursor = this.cursor;
    }

    const url = new URL(`${baseUrl}/twitch/videos`);
    url.searchParams.append("after", cursor);

    const res = await fetch(url);
    const result = await res.json();

    if (page === this.cursorPage && result.pagination?.cursor) {
      this.cursor = result.pagination?.cursor;
      this.cursorPage = page + 1;
    }

    return {
      data: result.data,
      pageInfo: {
        hasNextPage: result.pagination?.cursor,
        hasPreviousPage: false,
      },
    };
  },
} as unknown as DataProvider;

const baseDataProvider = combineDataProviders((resource) => {
  switch (resource) {
    case "twitchStreams":
      return twitchVideosDataProvider;
    default:
      return crudDataProvider;
  }
});

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
  async chat(messages: ChatMessage[]): Promise<ChatMessage[]> {
    const result = await fetch(`${baseUrl}/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(messages),
    });

    return result.json();
  },

  async getStreamClips(prefix: string) {
    const url = new URL(`${baseUrl}/stream_ingestion/find_files`);
    url.searchParams.append("prefix", prefix);

    return fetch(url).then((res) => res.json());
  },

  async getRenderedEpisodeFiles() {
    return fetch(
      `${baseUrl}/stream_ingestion/find_rendered_episode_files`
    ).then((res) => res.json());
  },

  async queueStreamTranscription({
    stream_id: streamId,
    ...payload
  }: TranscriptionAPIDetectInput) {
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

  async queueStreamSilenceDetection({
    stream_id: streamId,
    ...payload
  }: SilenceDetectionAPIDetectInput) {
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

  async getTask(taskUrl: string) {
    return fetch(taskUrl).then((res) => res.json());
  },

  async subscribeToTaskStatus(
    callback: WebSocketCallback
  ): Promise<() => void> {
    return GTWebSocket.getInstance(baseUrl).subscribe(callback);
  },

  async bulkCreate<T>(resource: string, data: T[]) {
    const res = await fetch(`${baseUrl}/records/${resource}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ records: data }),
    });
    return res.json();
  },

  // twitch functions
  async twitchLogin() {
    const result = await fetch(`${baseUrl}/twitch/login`);
    const url = (await result.json()).url;

    return url;
  },

  async twitchCallback(code: string) {
    await fetch(`${baseUrl}/twitch/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code }),
    });
  },

  async importStreams(streams: unknown[]) {
    return fetch(`${baseUrl}/records/streams`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ records: streams }),
    });
  },

  // youtube functions
  async youtubeLogin() {
    const result = await fetch(`${baseUrl}/youtube/login`);
    const url = (await result.json()).url;

    return url;
  },

  async youtubeCallback(code: string) {
    await fetch(`${baseUrl}/youtube/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code }),
    });
  },

  async uploadEpisodeToYoutube(video: YoutubeUploadTaskPayload) {
    return fetch(`${baseUrl}/youtube/upload`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(video),
    });
  },
};
