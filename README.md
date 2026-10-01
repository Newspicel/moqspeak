# moqspeak

**Media over QUIC Speak.** A TeamSpeak-style voice client written in Rust on
[zgui](https://github.com/zortax/zgui). Voice and screen sharing travel over
[Media over QUIC](https://github.com/moq-dev/moq) through Cloudflare's draft-16 MoQ relay. A
Cloudflare Worker runs the channels, presence and chat.

```
┌──────────────────── moqspeak (Rust, zgui) ────────────────────┐
│ server tree · info panel · chat tabs · options · screen view  │
│                                                               │
│ control  JSON over WebSocket ─────► Cloudflare Worker          │  server/
│                                     Durable Object per server  │  channels, clients, chat, pokes, moves
│ media    one MoQ broadcast/client ► Cloudflare MoQ relay       │  draft-16.cloudflare.mediaoverquic.com
│          track "audio":  Opus 48 kHz, a group per packet       │
│          track "screen": AV1, a group per keyframe             │
└───────────────────────────────────────────────────────────────┘
```

## Features

* **TeamSpeak handling.** Double-click a channel (or press Enter on it) to join. Drag a user, or
  yourself, onto a channel to move them. Channels collapse with their arrow or ←/→. Right-click
  for context menus (sub-channels, delete, poke, local mute, kick, move to my channel). Each
  channel shows how many users it holds.
* **Voice.** Opus at 40 kbit/s with in-band FEC and a per-speaker jitter buffer.
  * Echo cancellation with [sonora](https://crates.io/crates/sonora), a pure-Rust port of
    WebRTC's AEC3. The speaker mix is fed in as the far end.
  * Noise suppression with [nnnoiseless](https://crates.io/crates/nnnoiseless) (RNNoise).
  * Neural speech detection with [earshot](https://crates.io/crates/earshot), combined with a
    level threshold that you set against a live meter.
  * Push-to-talk (hold F1, or `` ` `` outside text fields) or continuous transmission.
  * Local volume (0–200 %) and local mute per user.
* **Devices.** Pick the microphone and speakers in Options, or follow the system default. They
  switch live.
* **Screen sharing.**
  * Captured with [xcap](https://crates.io/crates/xcap) at 15 fps and up to 1600 px wide.
  * Encoded to AV1 with [rav1e](https://crates.io/crates/rav1e) and decoded with
    [rav1d](https://crates.io/crates/rav1d).
  * Each keyframe starts a new MoQ group, so a viewer who joins late starts at the latest
    keyframe.
  * Watch a stream in the info panel or open it in its own window.
* **Roles.** User, moderator and admin, enforced by the Worker.
  * Moderators create and edit channels, move users and kick.
  * Admins also delete channels and assign roles (right-click a user → Role).
  * The first person on a new server becomes its admin. A server always keeps at least one admin.
* **Identity.** Each install has an Ed25519 key (`identity.key` next to the settings). The Worker
  sends a challenge, and the client signs it, so a role cannot be taken by copying an ID.
* **Look.** System, light and dark themes on zgui-ui, avatars that ring while someone talks, and
  [Lucide](https://lucide.dev) icons.

## Run

```sh
cargo run -p moqspeak --release
cargo run -p moqspeak --release -- moqspeak.newspicel.workers.dev/demo Julian      # auto-connect
cargo run -p moqspeak --release -- --bot moqspeak.newspicel.workers.dev/demo BeepBot Lobby
cargo run -p moqspeak --release -- --bot moqspeak.newspicel.workers.dev/demo Tv Lobby --share-pattern
```

The address is `<worker host>/<server name>`. Each server name is its own virtual server with its
own channel tree. `--bot` runs a headless participant that beeps every two seconds and prints who
it hears. `--share-pattern` also makes it share a moving test pattern.

On macOS, grant microphone access, and Screen Recording access for sharing, when asked. Settings
live in the platform config directory (`~/Library/Application Support/dev.moqspeak.moqspeak/` on
macOS).

The build needs the nightly toolchain pinned in `rust-toolchain.toml` (zgui requires it), plus
`cmake` and `nasm`. libopus and the AV1 assembly are built from source.

## Build the macOS app

```sh
cargo build -p moqspeak --release
packaging/macos/bundle.sh            # writes dist/moqspeak.app, signed ad hoc
cp -R dist/moqspeak.app /Applications/
```

`packaging/icon/build.sh` regenerates the icons from `packaging/icon/moqspeak.svg`.

## Releases

`.github/workflows/build.yml` builds and tests on Linux, macOS (arm64) and Windows for every
push. Push a `v*` tag to publish a GitHub release with:

* `moqspeak-linux-x86_64.tar.gz`
* `moqspeak-macos-arm64.dmg` and `.zip`. With the secrets below, these are signed with your
  Developer ID and notarized. Without them they are signed ad hoc.
* `moqspeak-windows-x86_64.zip`

### macOS signing and notarization

Set these repository secrets (Settings → Secrets and variables → Actions):

| Secret | What it is |
|---|---|
| `MACOS_CERTIFICATE` | Your **Developer ID Application** certificate and private key, exported from Keychain Access as `.p12`, then `base64 -i cert.p12 \| pbcopy` |
| `MACOS_CERTIFICATE_PASSWORD` | The password you gave the `.p12` export |
| `APPLE_API_KEY` | An App Store Connect API key (`AuthKey_XXXX.p8`, role *Developer*), base64-encoded |
| `APPLE_API_KEY_ID` | That key's ID |
| `APPLE_API_ISSUER_ID` | The issuer ID shown above the key list in App Store Connect → Users and Access → Integrations |

The workflow finds the signing identity in the certificate, signs with the hardened runtime and
the microphone entitlement, and notarizes and staples both the app and the DMG.

## Layout

| Path | What |
|---|---|
| `client/src/audio/` | cpal devices, Opus, mixer and jitter buffer, RNNoise + earshot processing |
| `client/src/screen/` | xcap capture, rav1e encode, rav1d decode, colour conversion, OBU fix-ups |
| `client/src/media.rs` | MoQ publish/subscribe (`moq-tokio`/`moq-net`, IETF draft-16) |
| `client/src/engine.rs` | tokio runtime: control WebSocket, subscriptions, sharing |
| `client/src/ui/` | zgui components, one per file, and `style.css` |
| `client/src/bot.rs` | the headless test participant |
| `server/` | the Cloudflare Worker + Durable Object |

## Server

```sh
cd server
pnpm install
wrangler deploy                 # https://moqspeak.newspicel.workers.dev
wrangler secret put MOQ_TOKEN   # a publish+subscribe token for the MoQ relay
```

**Rotating relay tokens.** A relay holds at most ten tokens, so tokens cannot be per session.
The `TokenVault` Durable Object hands every authenticated client the current token. It mints a
new 24-hour token when the current one has less than six hours left, and deletes expired ones.
Rotation turns on when the Worker has an API token:

```sh
wrangler secret put CF_API_TOKEN   # an account API token that can manage MoQ relay tokens
```

`CF_ACCOUNT_ID` and `MOQ_RELAY_ID` are set in `wrangler.jsonc`. Without `CF_API_TOKEN`, or when
minting fails (the Worker logs why), the static `MOQ_TOKEN` is used.

Mint the static relay token with
`cf realtime moq relays tokens create <relay-id> --operations publish subscribe`. The client
dials `https://draft-16.cloudflare.mediaoverquic.com/<token>`: Cloudflare reads the token from
the URL path. The relay ID is only used in the API.

## Caveats

* Anyone who can reach the Worker can create an identity and receive the current relay token.
  Rotation limits how long a leaked token works; it does not decide who may connect.
* The screen viewer shows a keyframe within two seconds of joining, and the AV1 encoder adds
  about four frames of latency.

## Licence

ISC, see `LICENSE`. Icons: [Lucide](https://lucide.dev), ISC licence (`client/assets/icons/LICENSE`).
