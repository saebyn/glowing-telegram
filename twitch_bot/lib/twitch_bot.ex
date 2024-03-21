defmodule TwitchBot.Bot do
  use GenServer

  # Starts the GenServer
  def start_link(_) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  # GenServer initialization
  def init(:ok) do
    {:ok, %{}}
  end
end
