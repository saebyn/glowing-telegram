defmodule TwitchBot.MessageParser do
  @spec parse_message(String.t()) :: {:ok, TwitchBot.Message.t()}
  def parse_message(message) do
    message =
      {message,
       %{
         :type => :unknown,
         :message => message
       }}
      |> parse_tags()
      |> parse_author()
      |> parse_type()
      |> parse_channel()
      |> parse_message_text()
      |> elem(1)

    {:ok,
     %TwitchBot.Message{
       author: message[:author],
       message: message[:message],
       type: message[:type],
       tags: message[:tags],
       channel: message[:channel]
     }}
  end

  defp parse_tags({message, message_map}) do
    case Regex.run(~r/@([^ ]*)/, message) do
      nil ->
        {message, message_map}

      [_head | [tags | _rest]] ->
        message_map =
          tags
          |> String.split(";")
          |> Enum.reduce(%{}, fn tag, acc ->
            [key, value] = String.split(tag, "=")
            key = String.replace(key, ~r/-/, "_")
            Map.put(acc, String.to_atom(key), value)
          end)
          |> then(fn tags ->
            Map.put(message_map, :tags, tags)
          end)

        {String.replace(message, ~r/@([^ ]*)/, ""), message_map}
    end
  end

  defp parse_author({message, message_map}) do
    case Regex.scan(~r/:(.*)!/, message) do
      [] ->
        {message, message_map}

      [[_, author]] ->
        {
          String.replace(message, ~r/:(.*)!/, ""),
          Map.put(message_map, :author, author)
        }
    end
  end

  defp parse_type({message, message_map}) do
    case Regex.scan(~r/PRIVMSG/, message) do
      [] ->
        {message, message_map}

      _ ->
        {
          String.replace(message, ~r/PRIVMSG/, ""),
          Map.put(message_map, :type, :privmsg)
        }
    end
  end

  defp parse_channel({message, message_map}) do
    case Regex.scan(~r/#([^ ]*)/, message) do
      [] ->
        {message, message_map}

      [[_, channel]] ->
        {String.replace(message, ~r/#([^ ]*)/, ""), Map.put(message_map, :channel, channel)}
    end
  end

  defp parse_message_text({message, message_map}) do
    {message,
     Map.put(
       message_map,
       :message,
       # remove everything before the first colon
       message
       |> String.split(":", parts: 2)
       |> List.last()
       |> String.trim()
     )}
  end
end
