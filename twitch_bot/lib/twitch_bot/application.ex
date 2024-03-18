defmodule TwitchBot.Application do
  use Application

  def start(_type, _args) do
    children = [
      # Starts a worker by calling: TwitchBot.Bot.start_link([])
      {TwitchBot.Bot, []}
    ]

    # Specify strategy as :one_for_one if you want the supervisor to restart
    # your bot in case it crashes.
    opts = [strategy: :one_for_one, name: TwitchBot.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
