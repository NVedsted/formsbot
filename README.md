# Formsbot
A simple Discord bot that makes it possible to create forms using Discord modals which are then submitted to a private thread.

# Features
- Setup forms through the Discord modal feature
- Submit responses to private threads
- Create and edit everything through Slash Commands
- Limit how often a user can submit a form through cooldowns

# Quickstart
The bot requires a Redis server and must be provided the following environment variables:
| Environment variable | Description |
|----------------------|-------------|
|`DISCORD_TOKEN`|The Discord bot token|
|`REDIS_URL`| The URL to a Redis server ([format](https://docs.rs/redis/latest/redis/#connection-parameters)) |

The bot can be built/run from source with `cargo build`/`cargo run`. Alternatively, a Docker image is provided which can be pulled like so:
```
docker pull ghcr.io/nvedsted/formsbot:latest
```

If you want a Docker Compose setup, [compose.yaml](compose.yaml) can serve as inspiration.

# Usage
The bot utilizes slash commands and should be rather intuitive to use. A simple flow might look something like this:
1. Use `/forms create` to create a new form
2. Use `/forms fields add` to add fields
3. Use `/forms show` to check that everything is in order
4. Use `/forms button` to create a button to open the form
5. The form is now ready for use!

If you encounter problems or just have a great idea for the bot, [feel free to create an issue](https://github.com/NVedsted/formsbot/issues/new)!
