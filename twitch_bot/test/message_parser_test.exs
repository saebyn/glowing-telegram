defmodule MessageParserTest do
  use ExUnit.Case
  alias TwitchBot.MessageParser
  doctest TwitchBot.MessageParser

  test "can parse a complicated message" do
    message =
      "@badge-info=founder/3;badges=founder/0,sub-gifter/1;client-nonce=f67031426d5a2985ceb14d1ba5351b49;color=#3BC43B;display-name=BrainlessSociety;emotes=;`;tmi-sent-ts=1711302722640;turbo=0;user-id=178536615;user-type= :brainlesssociety!brainlesssociety@brainlesssociety.tmi.twitch.tv PRIVMSG #saebyn :doctest will run the tests in the documentation"

    assert MessageParser.parse_message(message) ==
             {:ok,
              %TwitchBot.Message{
                tags: %{
                  badge_info: "founder/3",
                  badges: "founder/0,sub-gifter/1",
                  client_nonce: "f67031426d5a2985ceb14d1ba5351b49",
                  color: "#3BC43B",
                  display_name: "BrainlessSociety",
                  emotes: "",
                  tmi_sent_ts: "1711302722640",
                  turbo: "0",
                  user_id: "178536615",
                  user_type: ""
                },
                type: :privmsg,
                author: "brainlesssociety",
                message: "doctest will run the tests in the documentation"
              }}
  end
end
