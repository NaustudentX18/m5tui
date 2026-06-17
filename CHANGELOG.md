# Changelog

All notable changes to m5Tui will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- README.md and ROADMAP.md now explicitly distinguish structural completion (trait stubs, tests, CI) from functional completion (real hardware/network integration). Stub-to-real work begins.


## [0.1.0] - 2026-06-15 — M0 Foundation

### Added
- Cargo workspace with two member crates: `m5tui-core` (library) and `m5tui-bin` (binary shim). Zero external deps for M0.
- `m5tui-core`: pure `AppState` + `reduce(state, event) -> state` reducer (no I/O in the reducer).
- `m5tui-core`: `Frame` (40x16 cells) + `Cell { glyph, fg, bg, attrs }` RGB565 framebuffer model.
- `m5tui-core`: `render(&AppState) -> Frame` — centres the title string horizontally + vertically.
- `m5tui-core`: `sim::render_to_rgba(&Frame) -> Vec<u8>` — converts the frame to a 240x135 RGBA8888 buffer for golden-diff testing. The cell grid covers 240x128; the bottom 7 rows of the physical screen are background.
- `m5tui-core`: 5x8 ASCII glyph atlas (96 glyphs) hand-rolled as a `const` table; the 10 unique glyphs in "m5Tui v0.1.0" are drawn, the rest are blank.
- `crates/m5tui-bin/src/main.rs` — thin shim calling `m5tui_core::run()`.
- Layout constants: `COLS=40`, `ROWS=16`, `CELL_W=6`, `CELL_H=8`, `FB_W=240`, `FB_H=135` (physical screen), `GRID_H=128` (cell grid height).
- Unit tests in every module (reducer transitions, layout math, frame invariants, render correctness).
- Integration test `tests/sim_cockpit.rs` — asserts the sim output is 129600 bytes, deterministic, and has opaque pixels at the title position.
- `.github/workflows/ci.yml` — fmt + clippy + test on ubuntu-latest + macos-latest.
- `.github/workflows/xtensa.yml` — STUB for M1. Will activate when `crates/m5tui-device/` lands. Commented `if:` line shows the activation rule.
- `.github/dependabot.yml` — weekly Cargo + GitHub Actions update PRs.
- `LICENSE` — MIT, © 2026 Forest Hudson.
- `rust-toolchain.toml` — pins `stable` + `rustfmt` + `clippy`.

### Changed
- `PLANNING.md` — status flipped from "🟡 AWAITING SWAT APPROVAL" to "🟢 SWAT APPROVED — 2026-06-15".
- `ROADMAP.md` — M0 flipped to "✅ DONE — 2026-06-15" in the progress bar (1 of 11 milestones done).
- `README.md` — status block flipped to "🟢 M0 SHIPPED — 2026-06-15".

### Known limitations
- **No device build yet.** The xtensa CI workflow is a stub; it does not run an actual `xtensa-esp32s3-espidf` build. This activates in M1 when `crates/m5tui-device/` is created.
- **No on-device toolchain on the Pi.** The orchestrator chose not to install `espup`/ESP-IDF locally (1.5–2 GB download). Use GitHub Actions for xtensa until a self-hosted runner or local toolchain is set up. Recipe in `HARDWARE.md` §9 (TBD by M1).
- **No themes yet.** M0 only paints the static title in cyan. The 6 themes and the on-device editor land in M2.
- **No SSH, OMP, voice.** All the other crates in `ARCHITECTURE.md §2` are M3–M5b work.

## [0.2.0] - 2026-06-16 — M1 Cockpit shell + M2 Theme engine

### Added
- M1: `m5tui-core` input layer: `keymap` with 56-key Cardputer-Adv matrix, `ChordParser`, and 12 semantic `KeyAction` verbs.
- M1: `m5tui-core` widgets: `cockpit` (top bar + agent list + session pane + prompt + hint), `palette` (12 built-in commands, fuzzy matcher), `help` (two-column hotkey overlay), and `toast` overlay.
- M1: `palette::Command` descriptors + pure fuzzy matcher (`fuzzy_score`/`filter`) with no external deps.
- M1: `Event`/`Outgoing`/`Focus`/`Mode` enums and `step()` side-channel reducer returning `(AppState, Vec<Outgoing>)`.
- M2: new `m5tui-themes` crate: hand-rolled YAML parser, `Theme` schema, RGB565 conversion, and semantic validation.
- M2: six built-in themes (`coldwire`, `phosphor`, `lacuna`, `magline`, `noctilux`, `ivoryroom`) under `themes/`.
- M2: `m5tui-core::default_theme()` and `render(&AppState, &Theme)` — all widgets are now theme-aware and read colors from `theme.palette`.
- M2: `m5tui-core` widget `theme_editor` (9-screen overlay with live cockpit preview) and `;t` chord dispatch to `Mode::ThemeEditor`.
- M2: sim golden tests `sim_themes`, `sim_theme_editor`, `sim_theme_invalid`, plus coverage of every built-in theme.

### Changed
- `render(&AppState)` → `render(&AppState, &Theme)` across `m5tui-core`, `m5tui-bin`, and integration tests.
- `m5tui-bin` now renders and prints Cockpit, Palette, Help, and ThemeEditor in sequence.
- `m5tui-core/src/palette.rs` is now the M1 fallback palette; live colors come from `m5tui-themes::ThemePalette`.
- `ROADMAP.md` progress bar: M0 ✅, M1 ✅, M2 ✅ (3 of 11 done).

### Known limitations
- **Theme editor is view-only.** The 9 sub-screens (palette swatches, glyphs, layout, etc.) and SD-card save/export are M2.x polish not included in this commit.
- **No device build yet.** `crates/m5tui-device/` and the `xtensa-esp32s3-espidf` target are still M3 work.
- **No SSH/OMP/voice/market/handoff.** M3–M6 remain on the roadmap.

[0.1.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v0.1.0

## [1.0.0] - 2026-06-16 — Structural v1.0

### Added
- M3 crates: `m5tui-ssh`, `m5tui-profile`, `m5tui-persist` with trait stubs,
  in-memory implementations, and unit tests.
- M4 crate: `m5tui-omp` with line-oriented frame codec, `OmpSession` trait,
  and `StubOmpSession`.
- M5 crate: `m5tui-voice` with `AudioIn`/`AudioOut`, WAV encoder/decoder,
  PTT state machine, and `VoiceInbox`.
- M5a crate: `m5tui-book` + `book/*.yaml` sample spells, registry, template
  expansion, undo stack.
- M5b crate: `m5tui-market` + catalog schema, stub client, offline cache
  placeholder, theme preview via `m5tui-themes::parse`.
- M6 crate: `m5tui-handoff` with handoff store, minimal Markdown renderer,
  and vault search stub.
- `crates/m5tui-bin/tests/integration_v1.rs` smoke test exercising every
  crate through public APIs.
- Bumped every crate version to `1.0.0`.

### Changed
- `README.md` status block updated to v1.0.0 structurally shipped.
- `ROADMAP.md` progress bar: all 11 milestones marked DONE.

### Known limitations
- **Everything beyond M2 is a trait stub.** Real network (SSH/Tailscale),
  real hardware (I2S, SD atomic writes), real OMP RPC pipe, real market
  HTTPS backend, and real vault search are deferred to operator-side
  integration.

[0.1.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v0.1.0
[0.2.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v0.2.0
[1.0.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.0.0
