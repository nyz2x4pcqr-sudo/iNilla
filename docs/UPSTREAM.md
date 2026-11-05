# iNilla and upstream benilla

iNilla is a fork of [samwhosung/benilla](https://github.com/samwhosung/benilla) that **merges**
upstream as it publishes (the way [wenilla](https://github.com/Arnesen/wenilla) does). iNilla's own
code lives in its own crates and in `crates/benilla-app/src/inilla/`; changes inside upstream
files ("carries") are kept few and small so a merge conflicts on as little as possible.

## Syncing

```sh
git remote add upstream https://github.com/samwhosung/benilla   # once
git fetch upstream
git checkout -b sync-upstream-$(date +%Y%m%d)
git merge upstream/main          # a real merge commit — never a rebase or a squashed copy
# resolve the carries below if they conflict, then:
cargo test -p inilla-core && cargo check -p benilla-app
source ios/toolchain/linux-env.sh && cargo check --target aarch64-apple-ios -p inilla-ios
```

Open a PR from the sync branch and merge it **with a merge commit** (keeping upstream as the second
parent is what keeps the next sync small).

## The carries

Every upstream file iNilla changes, and why. Each change is marked `iNilla` in a comment.

| file | change |
|---|---|
| `Cargo.toml` | `[profile.ios]` (release, no LTO, symbols kept) |
| `crates/benilla-app/Cargo.toml` | depends on `inilla-core`, `inilla-platform`, `ureq`, `serde_json`; `bevy_egui` no longer optional; `arboard` not on iOS; the Wayland clipboard's `cfg` excludes iOS; `raw-window-handle` and `winit` on iOS |
| `crates/benilla-app/src/lib.rs` | `mod inilla`, `pub use inilla_boot`, `add_plugins(inilla::InillaPlugin)` |
| `crates/benilla-app/src/textinput/clipboard.rs` | the iOS pasteboard backend |
| `crates/benilla-app/src/net/io.rs` | the world address passes through `inilla::world_address` (the realm override) |
| `crates/benilla-app/src/player/camera.rs` | `LookConfig::yaw_rate`/`pitch_rate` `pub(crate)` (controller look in radians) |
| `crates/benilla-app/src/debug_panel/mod.rs` | the egui lane's demand gate also reads `inilla::EguiWanted` |
| `crates/benilla-ui/src/script/mod.rs`, `script/plain.rs` | `eval_plain` (wenilla's) — Lua results as plain values for the console, the HUD icons and the launcher's screen check |
| `.github/workflows/close-prs.yml` | removed (upstream closes every PR; this repository takes them) |
| `crates/benilla-app/src/addon_harness/render_tests.rs`, `use_probe_tests.rs` | the two tests whose fixtures are symlinks are `cfg(unix)`, so the app's tests build on Windows |

### Carries retired by the 2026-11-04 sync

Upstream now does these itself, so they were dropped instead of merged:

- **The binding seam** (`bindings.rs` `synth_*`, `Held::Synth`). Upstream moved the latches into
  `BindingLatches` and runs binding bodies with the call host lent. `inilla::synth::apply` now
  runs inside the binding pass (`BindingHead` → `BindingTail`) through `HostedVm`, keeps its own
  latches, and calls `execute_binding` / `CameraZoomIn(amount)` like a key — no upstream change.
- **The login hook** (`login/mod.rs`). Upstream removed the native login screen. The launcher now
  runs the stock `DefaultServerLogin(account, password)` in the glue VM once `AccountLogin` is
  shown (`inilla::session::submit_queued_login`, `inilla_core::lua::quote`).
- **The vendored C Lua's `os.execute`** (`loslib.c`). Upstream replaced C Lua with `benilla-lua`,
  which has no `os.execute`.
- **The game data's language** (`localize.rs`, `chain.rs`, `auth.rs`, `GetLocale`,
  `SetEuropeanNumbers`). Upstream reads the client locale from `Wow.ini` (`benilla-formats`
  `wow_ini`), sends it in the logon challenge, answers `GetLocale` with it, localizes DBC text and
  implements `SetEuropeanNumbers`. Difference: iNilla guessed the language from `ChrRaces.dbc`;
  upstream needs `Wow.ini` (`[WoW Config]` `Language`/`Country`) and is `enUS` without one.

Upstream has no CI and never builds iOS, so an upstream change can break only the iOS build; the
`inilla-check` workflow's iOS type-check is what catches it.
