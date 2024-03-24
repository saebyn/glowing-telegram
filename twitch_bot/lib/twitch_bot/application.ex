defmodule TwitchBot.Application do
  use Application

  @spec start(any(), any()) :: {:error, any()} | {:ok, pid()}
  def start(_type, _args) do
    children = [
      {TwitchBot.Websocket, []}
    ]

    opts = [strategy: :one_for_one, name: TwitchBot.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
