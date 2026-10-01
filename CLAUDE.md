# moqspeak

Voice chat client in the style of TeamSpeak. Rust, zgui, Media over QUIC. The control plane is a Cloudflare Worker.

## Workspace
- `client/` is the one Rust crate (`moqspeak`): engine, audio, media, screen share and all UI. `server/` is the Worker in TypeScript (`pnpm check`, `wrangler deploy`).
- zgui, zgui-ui, zgui-ui-tokens and zgui-ui-icons are git dependencies on `github.com/zortax/zgui`, pinned to one `rev`. Every zgui crate moves to a new rev together, in its own commit.
- Add crates with `cargo add`. Never write a version from memory. Use the latest version unless a concrete reason forces an older one.
- Toolchain is pinned in `rust-toolchain.toml` (same nightly as `~/Projects/zgui`). Do not change it.
- `unsafe` lives only in the platform bridges under `screen/`. `ui/` has none.

## Modules and files
- `mod.rs` style for directory modules. `mod.rs` declares and re-exports; it holds no logic.
- Modularize by feature, not by kind. A feature owns its types, state, views and tests.
- Small files. One UI component per file. One responsibility per module.
- Split a file before it needs a section-separator comment.
- Express the domain in the type system (enums, newtypes, typed ids). Stop before the types cost more than they save.

## Comments and docs
- Short. ASD-STE100 style: one idea per sentence, active voice, present tense.
- Describe what is. Never describe what was there before, what bug was fixed, or why an alternative was rejected.
- No contrastive negation or negative parallelism ("not x, but y", "rather x than y").
- Never mention plans, specs or external documents in code comments or rustdoc.
- Every module starts with a one-paragraph `//!` that states its responsibility.

## Engine and state
- The UI reaches the engine through `Command`, the `AudioShared` atomics and the `Mixer`. Network, codec and capture work never runs on the UI thread.
- Engine events reach the UI on one channel and are applied in `AppState::apply`. Signals are written from the UI thread only.
- `AppState` is `Copy` and provided as local context. Persisted preferences live in `Settings` and are saved on change.
- Never move a `!Send` UI handle into a thread or task.
- Hold `RenderEffect`, timer and `FrameHandle` handles for their lifetime. Clean up with `on_cleanup_local`.

## UI
- Design language: compact, flat and quiet. Dense rows, ghost controls that show a tone only under the pointer, one accent color for selection and focus, muted secondary text, no chrome.
- Check zgui-ui before writing a control. Use shipped components with custom styling. Build custom behavior when zgui-ui does not cover it.
- Generally useful controls and framework fixes go upstream into `~/Projects/zgui`. Never push there.
- Native elements: `on:pointer_down` for activation where it makes the app feel faster, `on:click` where a press must be cancelable. Read `ev.modifiers` for ctrl/shift logic.
- Styling: app sheets in `client/assets/css/` for values two components share; scoped `style!` per component where a sheet belongs to one component; `data-*` attributes for state CSS selects on. Class names are `ms-` BEM.
- Themes are CSS files of `--zui-*` and `--ms-*` declarations in `client/assets/themes/<name>-{light,dark}.css`. No Rust converts a theme into colors. Adding a theme is two files and one line in `ui/theme/variant.rs`.
- Status marks are CSS dots with tone tokens, never icons.
- Icons: lucide SVGs vendored in `client/assets/icons/`, referenced through `icons::NAME` constants. Stroke only, `currentColor`, untransformed.
- Row height is one constant in Rust and one `--ms-row` variable in CSS.
- Motion: CSS transitions and keyframes of 120 ms or less, motion tokens only. No timer-driven animation.
- Compact and flat: planes differ by tone, borders only on floating surfaces.

## Verify
- `cargo fmt --check`, `cargo clippy -p moqspeak`, `cargo test -p moqspeak --locked`.
- Test state logic with plain unit tests. Run the client against `moq.newspicel.dev/public` with a `--bot` beside it to see a populated server.
- Commit messages: short, active voice, describe the change. No references to plans or specs.
