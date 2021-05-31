# twch

[![build](https://github.com/mosmeh/twch/workflows/build/badge.svg)](https://github.com/mosmeh/twch/actions)

Twitch chat viewer in terminal

## curl version

```bash
cp .env.sample .env
vi .env
cargo run -p twch-server
```

```bash
curl localhost:8080               # Show popular streams
curl localhost:8080/search?q=foo  # Search active streams
curl localhost:8080/bar           # View chats of channel "bar"
```

## Standalone version

```bash
cp .env.sample .env
vi .env
cargo run -p twch-cli                # Show popular streams
cargo run -p twch-cli -- search foo  # Search active streams
cargo run -p twch-cli -- view bar    # View chats of channel "bar"
```
