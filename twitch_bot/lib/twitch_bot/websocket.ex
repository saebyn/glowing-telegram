defmodule TwitchBot.Websocket do
  alias TwitchBot.MessageParser
  use WebSockex
  require Logger

  @twitch_websocket_url "wss://irc-ws.chat.twitch.tv"

  def start_link(opts \\ []) do
    state = %{
      :username => System.get_env("TWITCH_USERNAME", "saebyn"),
      :channel => System.get_env("TWITCH_CHANNEL", "saebyn"),
      :token => System.get_env("TWITCH_TOKEN")
    }

    # add module name to the opts to name the process
    opts = Keyword.put_new(opts, :name, __MODULE__)

    {:ok, pid} =
      WebSockex.start_link(
        @twitch_websocket_url,
        __MODULE__,
        state,
        opts
      )

    {:ok, pid}
  end

  @spec send_message(pid(), String.t()) :: :ok
  def send_message(client, message) do
    Logger.info("Sending message")
    WebSockex.send_frame(client, {:text, message})
  end

  def handle_connect(_conn, state) do
    Logger.info("Connected!")

    pid = self()

    spawn(fn ->
      Logger.info("Joining channel")

      WebSockex.send_frame(
        pid,
        {:text, "CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership"}
      )

      WebSockex.send_frame(pid, {:text, "PASS oauth:#{state[:token]}"})
      WebSockex.send_frame(pid, {:text, "NICK #{state[:username]}"})
      WebSockex.send_frame(pid, {:text, "JOIN ##{state[:channel]}"})

      Logger.info("Joined channel")

      WebSockex.send_frame(pid, {:text, "PRIVMSG ##{state[:channel]} :Hello from the bot!"})
    end)

    {:ok, state}
  end

  def handle_frame({:text, "PING :tmi.twitch.tv" <> rest}, state) do
    Logger.info("Received PING: #{rest}")
    {:reply, {:text, "PONG :tmi.twitch.tv" <> rest}, state}
  end

  def handle_frame({:text, msg}, state) do
    Logger.info("Received message")

    case MessageParser.parse_message(msg) do
      {:ok, message} ->
        Logger.info("Message: #{inspect(message)}")

      {:unknown, _} ->
        nil
    end

    {:ok, state}
  end

  def handle_disconnect(%{reason: {:local, reason}}, state) do
    Logger.info("Local close with reason: #{inspect(reason)}")
    {:ok, state}
  end

  def handle_disconnect(disconnect_map, state) do
    super(disconnect_map, state)
  end
end
