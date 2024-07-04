/**
 * Websocket connection to the server to receive real-time updates.
 *
 * This module exports a class `GTWebSocket` that connects to the server's
 * websocket endpoint and allows you to subscribe to real-time updates.
 * The class is a singleton, so you can only have one instance of it.
 * To get the instance, use the `getInstance` method.
 */
import { TaskStatus } from "./types";

export interface TaskStatusWebsocketMessage {
  task: unknown;
  previous_status: TaskStatus;
  new_status: TaskStatus;
  event: "task_status_change";
}

export type Callback = (message: TaskStatusWebsocketMessage) => void;

interface Subscription {
  callback: Callback;
  id: number;
}

/**
 * Websocket connection to the server to receive real-time updates.
 */
export default class GTWebSocket {
  private ws?: WebSocket;
  private subscriptions: Subscription[] = [];
  private maxId: number = 0;

  static getInstance(baseUrl: string) {
    console.log("Getting websocket instance 1️⃣", {
      baseUrl,
      websocket: window.websocket,
    });

    if (window.websocket === undefined) {
      window.websocket = new GTWebSocket(baseUrl);
    }

    return window.websocket;
  }

  private constructor(baseUrl: string) {
    this.ws = this.connect(baseUrl);
  }

  private connect(baseUrl: string): WebSocket {
    console.log("Subscribing to tasks websocket 2️⃣");
    const ws = new WebSocket(
      `${baseUrl.replace("http", "ws")}/records/tasks/ws`
    );

    ws.addEventListener("open", function (event) {
      console.log("Connected to tasks websocket", event);
      this.send(JSON.stringify({ event: "subscribe" }));
    });

    const messageEventListener = (event: MessageEvent) => {
      console.log("Message from server ", event.data);
      const message = JSON.parse(event.data);

      this.subscriptions.forEach((subscription) => {
        subscription.callback(message);
      });
    };

    ws.addEventListener("message", messageEventListener);

    ws.addEventListener("close", (event) => {
      console.log("Disconnected from tasks websocket, will reconnect", event);

      // Reconnect to the websocket
      this.ws = this.connect(baseUrl);
    });

    ws.addEventListener("error", (event) => {
      console.error("Error from tasks websocket", event);
    });

    return ws;
  }

  close() {
    this.ws?.close();
    this.subscriptions = [];
  }

  subscribe(
    callback: (message: TaskStatusWebsocketMessage) => void
  ): () => void {
    const id = this.maxId++;
    this.subscriptions.push({
      callback,
      id,
    });

    return () => {
      this.subscriptions = this.subscriptions.filter((s) => s.id !== id);
    };
  }
}

// Add a websocket key to the Window interface to store the websocket connection.
declare global {
  interface Window {
    websocket?: GTWebSocket;
  }
}
