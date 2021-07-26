# Bot Destroyer 9000

This bot was created out of a need to block raid attempts by scammers on Discord. They'd always join the server in
batches of 10 or 20 users, so it is relatively easy to counter. Use bots to fight bots!

When starting, the bot token is expected to be in the environment variable "DISCORD_TOKEN".

## Enable Logging

You enable logging through environment variables.

Best common usage:
```
RUST_LOG=bot_destroyer_9000=info bot-destroyer-9000.exe
```

See [env_logger](https://crates.io/crates/env_logger) for more info.
