defmodule TwitchBot.Message do
  @enforce_keys [:type, :message]
  defstruct(author: nil, message: "", type: :unknown, tags: %{}, channel: nil)
  @type t :: %__MODULE__{author: String.t(), message: String.t()}

  @type type ::
          :privmsg
          | :notice
          | :part
          | :join
          | :ping
          | :userstate
          | :roomstate
          | :globaluserstate
          | :unknown
end
