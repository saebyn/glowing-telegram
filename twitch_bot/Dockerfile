ARG ELIXIR_VERSION
FROM elixir:${ELIXIR_VERSION:-latest}


WORKDIR /app

# Copy the current directory contents into the container
COPY . .

# Install dependencies
RUN mix local.hex --force
RUN mix local.rebar --force
RUN mix deps.get

# Compile the project
RUN mix do compile

ENV MIX_ENV=prod
ENV TWITCH_BOT_USERNAME=your_bot_username
#ENV TWITCH_BOT_OAUTH=UNSET

# Command to run the application
CMD ["mix", "run", "--no-halt"]
