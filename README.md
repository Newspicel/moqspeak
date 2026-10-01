# moqspeak

A TeamSpeak 3 style voice client, native on [zgui](../zgui), with voice carried over
[Media over QUIC](https://github.com/moq-dev/moq) through Cloudflare's draft-16 MoQ relay.

```
┌──────────────── moqspeak (Rust, zgui) ────────────────┐
│ Menu · toolbar · server tree · info panel · chat · bar │
│                                                        │
│  control: WebSocket JSON ──────────► Cloudflare Worker  │  server/  (Durable Object per server:
│                                      wss://…/s/<name>   │           channels, clients, chat, pokes)
│  voice:  Opus 20 ms frames ────────► MoQ relay          │  draft-16.cloudflare.mediaoverquic.com
│          moq-net, IETF draft-16      (relay 615ec116…)  │  token minted for this relay, served by
│          one broadcast per client,                      │  the Worker in the welcome message
│          track "audio", group/packet                    │
└────────────────────────────────────────────────────────┘
```

## Run

```sh
cargo run -p moqspeak --release                                  # opens the client
cargo run -p moqspeak --release -- moqspeak.newspicel.workers.dev/demo Julian   # auto-connect
cargo run -p moqspeak --release -- --bot moqspeak.newspicel.workers.dev/demo BeepBot Lobby
```

The address is `<worker host>/<server name>`. Every server name is its own virtual server with
its own channel tree (seeded with Lobby, Gaming → Counter-Strike/Minecraft/…, AFK, …).
`--bot` runs a headless participant that beeps every two seconds and prints who it hears, which
is handy for testing with one machine.

### In the client

* **Double-click a channel** to switch to it. You hear everyone in your channel.
* **Double-click a client** (or right-click → Send Text Message) for a private chat tab.
* **Right-click** for TS3-style context menus: create sub-channel, delete channel, poke, mute
  locally, kick from channel.
* **Toolbar**: connect, disconnect, connect to the public bookmark, mute microphone, mute
  speakers, away, create channel, options.
* **Tools → Options**: voice activation with a live level meter and threshold, push-to-talk
  (hold **F1**, or **`** outside text fields, or the toolbar button), continuous transmission,
  microphone gain, and output volume.
* **Tools → Microphone test** plays your own voice back to you.
* Each client in the info panel has a local volume slider (0–200 %).
* Settings and bookmarks are stored in `~/Library/Application Support/dev.moqspeak.moqspeak/`.

## Layout

| Path | What |
|---|---|
| `client/src/audio.rs` | cpal capture/playback, VAD/PTT gate, Opus encode, per-speaker jitter buffer + mixer |
| `client/src/media.rs` | MoQ publish/subscribe with `moq-tokio`/`moq-net` (two QUIC sessions: publish, subscribe) |
| `client/src/engine.rs` | tokio runtime: control WebSocket, subscription sync, talking detection |
| `client/src/model.rs` | the JSON control protocol |
| `client/src/ui/` | zgui components (one per file) and `style.css` |
| `client/src/bot.rs` | the headless test participant |
| `server/` | the Cloudflare Worker + Durable Object |

## Server

```sh
cd server
pnpm install
wrangler deploy                 # deployed at https://moqspeak.newspicel.workers.dev
wrangler secret put MOQ_TOKEN   # a publish+subscribe token for the MoQ relay
```

The relay token was minted with
`cf realtime moq relays tokens create 615ec116435c0bb9c0747dbdea2b495c --label moqspeak-worker --operations publish subscribe`
and stored as the `MOQ_TOKEN` secret without being printed. The relay URL the client dials is
`https://draft-16.cloudflare.mediaoverquic.com/<token>`. Cloudflare reads the token from the
URL path and the relay ID only appears in the API.

`GET /s/<name>` returns a JSON snapshot of a server, which is useful for debugging.

## Caveats

* Every client that reaches the Worker receives the same relay token. That is fine for a demo.
  Before a public deployment, mint short-lived per-session tokens from the Worker through the
  Cloudflare API.
* The Worker trusts its clients: anyone can create/delete channels and kick. There are no
  permissions or server groups yet.
* Discovery uses `SUBSCRIBE_NAMESPACE`/`PUBLISH_NAMESPACE` on the relay. A newly joined speaker is
  heard after about one second.
* There is no echo cancellation. Use headphones or push-to-talk.
