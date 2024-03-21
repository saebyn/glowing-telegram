defmodule TwitchBot.Websocket do
  use WebSockex
  require Logger

  def start_link(state) do
    {:ok, pid} =
      WebSockex.start_link("wss://irc-ws.chat.twitch.tv", __MODULE__, state, debug: [:trace])

    {:ok, pid}
  end

  def handle_connect(conn, state) do
    Logger.info("Connected to Twitch IRC server at #{conn.host}:#{conn.port}")
    Logger.debug("State: #{inspect(conn)}")

    send(self(), :subscribe)

    {:ok, state}
  end

  def handle_disconnect(connection_status_map, state) do
    Logger.error("Disconnected from Twitch IRC server")
    Logger.debug("Connection status map: #{inspect(connection_status_map)}")

    {:ok, state}
  end

  def handle_info(:subscribe, state) do
    Logger.info("Subscribing to Twitch IRC server")
    # send_text("PASS #{state[:oauth_token]}")
    send_text("NICK #{state[:username]}")
    send_text("JOIN #{state[:channel]}")
    # send_text("CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership")

    {:ok, state}
  end

  def handle_frame({:text, <<"PING :tmi.twitch.tv"::binary, _::binary>>}, state) do
    Logger.debug("Received PING from Twitch IRC server")
    send_text("PONG :tmi.twitch.tv")

    {:ok, state}
  end

  def handle_frame(frame, state) do
    Logger.debug("Received frame: #{inspect(frame)} from Twitch IRC server")
    {:ok, state}
  end

  def handle_cast({:send, frame}, state) do
    Logger.debug("Sending frame: #{inspect(frame)} to Twitch IRC server")
    {:reply, frame, state}
  end

  def terminate(reason, state) do
    Logger.error("Terminating Websocket connection: #{inspect(reason)} #{inspect(state)}")

    exit(:normal)
  end

  defp send_text(message) do
    Logger.debug("Sending text: #{message} to Twitch IRC server")
    GenServer.cast(__MODULE__, {:send, {:text, message}})
  end
end
