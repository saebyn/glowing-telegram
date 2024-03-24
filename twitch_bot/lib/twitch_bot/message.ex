defmodule TwitchBot.Message do
  @enforce_keys [:author, :message]
  defstruct [:author, :message]
  @type t :: %__MODULE__{author: String.t(), message: String.t()}
end
