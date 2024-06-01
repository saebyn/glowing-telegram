# config/runtime.exs
import Config
import Dotenvy

env = source!([".env", System.get_env()])

config :twitch_bot, TwitchBot.Websocket,
  token: env["TWITCH_TOKEN"],
  username: env["TWITCH_USERNAME"],
  channel: env["TWITCH_CHANNEL"]
