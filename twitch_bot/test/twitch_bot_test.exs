defmodule TwitchBotTest do
  use ExUnit.Case
  doctest TwitchBot

  test "greets the world" do
    assert TwitchBot.hello() == :world
  end
end
