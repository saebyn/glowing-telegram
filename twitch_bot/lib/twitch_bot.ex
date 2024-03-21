defmodule TwitchBot.Bot do
  use GenServer

  # Starts the GenServer
  def start_link(_) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  # GenServer initialization
  def init(:ok) do
    {:ok, _} = :application.ensure_all_started(:gun)

    connect_opts = %{
      protocols: [:http]
    }

    # Example of opening a WebSocket connection
    {:ok, conn_pid} =
      case :gun.open(~c"irc-ws.chat.twitch.tv", 443, connect_opts) do
        {:ok, conn_pid} ->
          IO.puts("Opened #{inspect(conn_pid)}")
          {:ok, conn_pid}

        {:error, reason} ->
          exit({:gun_open_failed, reason})
      end
      |> IO.inspect()

    # Wait for gun to be up - equivalent to :gun_up message
    {:ok, _http_version} = :gun.await_up(conn_pid) |> IO.inspect()

    stream_ref = :gun.ws_upgrade(conn_pid, ~c"/", []) |> IO.inspect()

    receive do
      {:gun_upgrade, ^conn_pid, ^stream_ref, ["websocket"], headers} ->
        upgrade_success(conn_pid, headers, stream_ref)

      {:gun_response, ^conn_pid, _, _, status, headers} ->
        exit({:ws_upgrade_failed, status, headers})

      {:gun_error, _conn_pid, _stream_ref, reason} ->
        exit({:ws_upgrade_failed, reason})

      whatever ->
        IO.inspect(whatever, label: "Whatever")
        # More clauses here as needed.
    after
      10000 ->
        IO.puts("Took too long!")
        :erlang.exit("barf!")
    end

    # Send authentication and join channel messages
    # :gun.ws_send(conn_pid, {:text, "PASS oauth:your_oauth_token\r\n"})
    # :gun.ws_send(conn_pid, {:text, "NICK your_username\r\n"})
    # :gun.ws_send(conn_pid, {:text, "JOIN #channel_name\r\n"})

    {:ok, %{conn_pid: conn_pid}}
  end

  def upgrade_success(conn_pid, headers, _stream_ref) do
    IO.puts("Upgraded #{inspect(conn_pid)}. Success!\nHeaders:\n#{inspect(headers)}\n")
  end

  # Callbacks for WebSocket events, e.g., handling incoming messages
  def handle_info(_msg, state) do
    # Process incoming WebSocket messages here
    {:noreply, state}
  end

  # To stop gun
  def stop(conn_pid) do
    :gun.shutdown(conn_pid)
  end
end
