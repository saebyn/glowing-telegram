defmodule TwitchBot.Application do
  use Application

  @spec start(any(), any()) :: {:error, any()} | {:ok, pid()}
  def start(_type, _args) do
    children = [
      {TwitchBot.Websocket,
       %{
         :oauth_token => System.get_env("TWITCH_OAUTH_TOKEN"),
         :username => System.get_env("TWITCH_USERNAME", "saebyn"),
         :channel => System.get_env("TWITCH_CHANNEL", "saebyn")
       }},
      {TwitchBot.Bot, []}
    ]

    # Specify strategy as :one_for_one if you want the supervisor to restart
    # your bot in case it crashes.
    opts = [strategy: :one_for_one, name: TwitchBot.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
