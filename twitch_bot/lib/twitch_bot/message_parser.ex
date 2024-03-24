defmodule TwitchBot.MessageParser do
  def parse_message(message) do
    {message,
     %{
       :type => :unknown,
       :message => message
     }}
    |> parse_tags()
    |> parse_author()
    |> parse_type()
    |> parse_channel()
    |> elem(1)
  end

  defp parse_tags({message, message_map}) do
    case Regex.scan(~r/@([^ ]*)/, message) do
      [] ->
        {message, message_map}

      [tags] ->
        tags
        |> List.first()
        |> String.split(";")
        |> Enum.reduce(%{}, fn tag, acc ->
          [key, value] = String.split(tag, "=")
          Map.put(acc, String.to_atom(key), value)
        end)
        |> then(fn tags ->
          Map.put(message_map, :tags, tags)
        end)
        |> Map.put(:message, String.replace(message, ~r/@([^ ]*)/, ""))
    end
  end

  defp parse_author({message, message_map}) do
    case Regex.scan(~r/:(.*)!/, message) do
      [] ->
        {message, message_map}

      [[_, author]] ->
        Map.put(message_map, :author, author)
        |> Map.put(:message, String.replace(message, ~r/:(.*)!/, ""))
    end
  end

  defp parse_type({message, message_map}) do
    case Regex.scan(~r/PRIVMSG/, message) do
      [] ->
        {message, message_map}

      _ ->
        Map.put(message_map, :type, :privmsg)
        |> Map.put(:message, String.replace(message, ~r/PRIVMSG/, ""))
    end
  end

  defp parse_channel({message, message_map}) do
    case Regex.scan(~r/#([^ ]*)/, message) do
      [] ->
        {message, message_map}

      [[_, channel]] ->
        Map.put(message_map, :channel, channel)
        |> Map.put(:message, String.replace(message, ~r/#([^ ]*)/, ""))
    end
  end
end
