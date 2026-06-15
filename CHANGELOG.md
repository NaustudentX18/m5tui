# Changelog

All notable changes to m5Tui will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v0.1.0
