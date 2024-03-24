defmodule TwitchBot.MessageParser do
  def parse_message(message) do
    # TODO consider a pipeline
    # e.g. message |> split_message |> parse_message
    # or  {message, %{}} |> parse_tags() |> parse_whatever() |> elem(1)
    case String.split(message, " ", parts: 5) do
      [
        _tags,
        ":" <> author,
        "PRIVMSG",
        _channel,
        ":" <> message
      ] ->
        author = String.replace(author, ~r/!.*/, "")

        {:ok, %TwitchBot.Message{author: author, message: message}}

      _ ->
        {:unknown, message}
    end
  end

  def is_message?(message) do
    case parse_message(message) do
      {:unknown, _} -> false
      _ -> true
    end
  end
end
