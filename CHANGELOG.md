# Changelog

All notable changes to m5Tui will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.13.0] - 2026-06-17 — Stub-to-real wave 12: book registry, Markdown renderer, OMP widgets

### Added
- `m5tui-book::load_book_dir(dir) -> Result<InMemoryRegistry, BookError>` — real `std::fs::read_dir` walker that reads every `*.yaml` in the directory and parses it through a new hand-rolled `parse_spell_yaml`. Supports the 6 sample shapes (`plan`, `ask`, `git-status`, `ssh-status`, `summarize`, `weather-stub`): top-level scalars, `params:` list of multi-line bullet records, and `template:` / `undo_template:` as either a quoted scalar or a `|` block scalar with indent stripping.
- `m5tui-book::FileRegistry { dir, inner, mtimes }` — directory-watching spell registry. `new(dir)` records mtimes; `refresh_if_stale()` re-reads any YAML whose mtime changed (or appeared new) or was deleted; `spell_count()` for quick checks; `impl BookRegistry` delegating to the inner `InMemoryRegistry`.
- `m5tui-book::BookError::DuplicateName(String)` + `BookError::Io(String)` + `BookError::Parse { path, line, message }` + `BookError::io(action, path, e)` helper for ergonomic IO-error wrapping (mirrors `m5tui-persist::PersistError::io`).
- `m5tui-book::InMemoryRegistry::insert(Spell)` — returns `BookError::DuplicateName` on collision.
- `crates/m5tui-book/tests/registry_fs.rs` (NEW, 6 integration tests): two-file load, empty dir, invalid YAML, `FileRegistry::new` count, `FileRegistry::refresh_if_stale` after rewrite, `&dyn BookRegistry` dispatch through `FileRegistry`.
- `m5tui-handoff::LineStyle` — expanded with 6 new variants (`Heading`, `Bullet`, `Numbered`, `Quote`, `Code`, `Rule`, `Blank`, `Body`, `InlineCode`) while keeping the original variants for backwards compatibility (15 existing lib tests still pass unchanged).
- `m5tui-handoff::render_markdown(src)` — two-pass CommonMark-subset renderer (~390 lines). Classifies each line (blank/fence/HR/heading/quote/ordered/unordered/prose), then runs `apply_inline` to transform `**bold**` to `*foo*`, `*italic*` to `_foo_`, `` `inline code` `` to `foo`, `[text](url)` to `text (url)`. Every emitted line is wrapped to the 40-col framebuffer via `wrap_text` (word-wrap on spaces, hard-break at col 38 with `-` suffix for long words); list/quote continuations are indented to match marker width.
- `m5tui-handoff::FRAME_COLS: usize = 40` — public constant for callers that need to mirror the wrap budget.
- `m5tui-handoff::InMemoryVaultClient` — `Debug + Default + Clone` with `new(hits)`, `from_jsonl(stream)`, `len()`, `is_empty()`, and `impl VaultSearch` whose `query(q)` returns `Ok(corpus.clone())` regardless of `q`. Constructors reuse `parse_hits_jsonl` so `HandoffError::Parse` bubbles up on malformed input.
- `crates/m5tui-handoff/tests/markdown.rs` (NEW, 10 tests) + `crates/m5tui-handoff/tests/vault_inmemory.rs` (NEW, 4 tests).
- `m5tui-core::app::{OmpCard, OmpTodo, OmpSubagent, ProfileSummary, SpellSummary}` — state types for the OMP event widgets and the picker UIs.
- `m5tui-core::app::AppState` — 8 new fields: `omp_cards: Vec<OmpCard>`, `omp_todos: Vec<OmpTodo>`, `omp_subagents: Vec<OmpSubagent>`, `omp_thinking: Option<String>`, `omp_answers: VecDeque<String>` (cap 20), `profiles: Vec<ProfileSummary>`, `book_spells: Vec<SpellSummary>`, `picker_index: usize`. All default to empty in `Default::default()` (cockpit mode preserved per the v1.12 gotcha).
- `m5tui-core::event::Event::OmpFrameReceived(m5tui_omp::OmpFrame)` — the framework injects this from the OMP transport loop; the reducer pushes to the right `AppState` field by `OmpFrame` kind.
- `m5tui-core::event::KeyAction::{ProfileUp, ProfileDown, BookUp, BookDown}` — picker navigation keys (bound to `j/k` in `Mode::ProfilePicker` / `Mode::Book`).
- `m5tui-core::step` — new arms for `Event::OmpFrameReceived` (ingest: `ToolCall` → push card, `ToolResult` → close matching card, `TodoUpdate` → upsert, `Subagent` → push, `Thinking` → set, `Answer` → clear thinking + append to ring buffer), plus `ProfileUp/Down`, `BookUp/Down` (wraps the `picker_index`), and Enter in `Mode::{ProfilePicker, Book}` (emits `Outgoing::PickProfile(id)` / `Outgoing::RunSpell(id)` via the new generic `enter_picker<T, F>` helper).
- `m5tui-core::widgets::omp_cards` (NEW) — renders the right pane of the cockpit from OMP events: 1 line `think: …`, 4 todo rows (`[ ] foo` / `[x] foo`), 2 subagent rows (`> task`), 6 card rows (`▶ id tool(args)` for running, `✓ id tool` for ended), 1 most-recent-answer row. Truncated to 40 cols.
- `m5tui-core::widgets::profile_picker` (NEW) — list-driven profile picker. Title `Profiles` in accent, body listing `state.profiles` with `state.picker_index` highlighted in `theme.palette.ok`, empty state `No profiles — run ;n`, hint `j/k move  ⏎ select  esc back`.
- `m5tui-core::widgets::book_picker` (NEW) — same shape as `profile_picker` but lists `state.book_spells`. Hint: `j/k move  ⏎ cast  esc back`.
- `m5tui-core::widgets::cockpit` — right pane now dispatches to `omp_cards::render` when `state.omp_cards` / `state.omp_thinking` / `state.omp_todos` is non-empty; falls back to the existing `MockSession` render otherwise (so the 6 existing cockpit sim tests still pass unchanged).
- `m5tui-core::widgets::overlay` — `render_profile_picker` and `render_book` now delegate to the new picker widgets (the static placeholder bodies are gone).
- `m5tui-core/Cargo.toml` — adds `m5tui-omp = { path = "../m5tui-omp" }` so the new `OmpFrameReceived` event can carry the OMP frame type directly.
- `m5tui-omp/Cargo.toml` — **drops** the dead `m5tui-core = { path = "../m5tui-core" }` dep. The dep was unused in the omp src (no `use m5tui_core::*` anywhere) and was blocking `m5tui-core` from depending on `m5tui-omp`. Now `m5tui-core` can use `m5tui_omp::OmpFrame` directly. The dead-dep removal was the only cycle fix; no other workspace change was needed.
- `crates/m5tui-core/tests/{omp_cards,profile_picker,book_picker}.rs` (NEW, 3 integration tests × ~3 cases each = 8 sim tests).
- `m5tui-core::app_tests.rs` — new reducer tests for OMP frame ingestion (5 cases: ToolCall push, ToolResult close, TodoUpdate upsert, Thinking+Answer clear, picker index wrap) and the `omp_answers` 20-cap.

### Changed
- Workspace: 444 → 485 tests (+41). All gates green: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (485 passed, 41 suites).
- PLAN.md todo delta: 64/68 → **67/68** done. The 3 newly completed items:
  - M5a Book of Commands: `Registry` walks `book/*.yaml`, validates, hot-reloads (was: in-memory only).
  - M6 Handoff viewer: full Markdown renderer replaces the "tiny subset" stub.
  - M4 OMP integration: right pane now renders `tool_call` / `todo_update` / `subagent` / `thinking` cards instead of just `MockSession`.

### Known limitations
- `m5tui-book::parse_spell_yaml` is intentionally minimal — handles only the 6 sample spell shapes (no flow-style mappings, no anchors, no `!include`). If a future spell file needs any of those, the parser will need to grow or we should switch to `serde_yaml`.
- `BookError::Parse.line` is always `0` — the hand-rolled parser doesn't thread line numbers from inner errors. The file path is preserved on every parse failure, which is the more useful signal.
- `FileRegistry::refresh_if_stale` uses fs `mtime` only; on filesystems with second-granular timestamps an edit within the same second as `new()` will be missed until the next refresh.
- The OMP `OmpFrameReceived` event currently lives on `m5tui-core`'s `AppState` directly; a future refactor might wrap it in a `Subscription`/`Reducer` pattern, but this is the simplest path for v1.

## [1.12.0] - 2026-06-17 — Boot screen + About + Log viewer

### Added
- `m5tui-core::widgets::boot` — animated boot screen with ASCII logo, 7-stage progress bar, and status line. Rendered on startup (`AppState::booting()`); any key calls `Outgoing::SkipBoot` to dismiss. 8 tests.
- `m5tui-core::widgets::about` — version/build/themes/keybinds info overlay, opened via `;A` (or `;?` from inside `Mode::Help`). 6 tests + 3 keymap tests.
- `m5tui-core::widgets::log_viewer` + `app::LogBuffer` — in-memory ring buffer (default 200 lines) with scroll/follow/jump-top/jump-bottom/follow-toggle. Opened via `;L`. In-mode bindings: `j/k/g/G/f`. 14+ tests (8 widget + 6 LogBuffer).
- `m5tui-core::app::AppState::booting()` constructor — explicit boot-mode state for the host binary, leaving `AppState::default()` as the cockpit-state used by every existing reducer test.
- `m5tui-core::keymap::keymap_for_mode(mode, c)` — state-aware chord dispatcher for in-mode bindings (LogViewer `j/k/g/G/f`). `parse_chord_with_mode` — state-aware version of `parse_chord` so `;?` from inside `Mode::Help` opens About.
- `.github/workflows/xtensa.yml`: fixed the `rustup target add xtensa-esp32s3-espidf` step that was failing on the default `stable` toolchain. Now uses espup's installed toolchain (which is the correct one for xtensa).

### Changed
- Workspace: 395 → 444 tests (+49). All gates green.
- `.github/workflows/xtensa.yml` cross-compile job is now correct; v1.11.0's run was failing on a pre-existing CI config bug.

## [1.11.0] - 2026-06-17 — Real device drivers + market GitHub Pages backend

### Added
- `m5tui-device::drivers` (new module, 1057 lines): ST7789V2 SPI LCD driver (240x135), TCA8418 I2C keyboard matrix driver (Cardputer-Adv keymap), BMI270 I2C IMU driver, SD card SPI block-storage driver, ES8311 I2S audio codec driver, plus the `Transport` / `SpiBus` / `I2sBus` trait abstractions and the host-side `MockTransport` / `MockSpi` / `MockI2s` so the driver logic is unit-testable without hardware. 27 new tests.
- `m5tui-market::github_pages_html(catalog) -> String` — self-contained static HTML (no external assets) for the operator to commit to `docs/index.html`. Each `ThemeEntry` becomes a card with name, version, swatch, description, and download link.
- `m5tui-market::MarketPublisher` + `MarketPublishPlan` + `format_pr_body` — pure, no-network publish planner that produces the catalog entry, the asset YAML, the file paths, and a Markdown PR body the operator (or CI) feeds to `gh pr create`. 15 new tests.
- `scripts/market-publish.sh` — operator-facing publish pipeline: validates the theme YAML, opens a branch, edits `market/catalog.json` via inline Python, pushes, and opens a PR. Idempotent.

### Known limitations
- The 5 driver structs compile and unit-test on the host; the on-device `unsafe extern "C"` paths into `esp-idf-hal` are still pending the ESP-IDF feature-gate work (depends on the operator installing the toolchain).
- The market GitHub Pages backend assumes a future `market/catalog.json` and `market/assets/` directory; the script will create the branch but the operator must wire up the Pages site + B2/R2 bucket first.

## [Unreleased]

### Changed
- README.md and ROADMAP.md now explicitly distinguish structural completion (trait stubs, tests, CI) from functional completion (real hardware/network integration). Stub-to-real work begins.

## [1.10.0] - 2026-06-17 — RusshChannel + Keepalive + voice auto-push

### Added
- `m5tui-ssh::russh_client::RusshChannel` — real `Channel` impl
  backed by a russh exec/PTY handle. On the host simulator the
  read/write/resize/close path drains a test buffer; on the
  device, the on-device build's `RusshClient` runtime drives the
  russh channel.
- `m5tui-ssh::russh_client::Keepalive` — pure-data state tracker
  with `pong()`/`miss()` methods and `KEEPALIVE_INTERVAL_SECS = 15`
  / `KEEPALIVE_MAX_MISSES = 3` constants. Drives the keepalive ping
  loop and triggers reconnect after 3 missed pongs.
- `m5tui-voice::auto_push_command(enabled, host, path)` — builds
  the `scp` command for a single memo push to `~/voice-inbox/`;
  returns `None` when auto-push is disabled.
- `m5tui-voice::plan_auto_push(inbox, enabled, host)` — plans a
  batch of auto-push commands for every memo with `pushed == false`.
  Returns an empty list when auto-push is disabled.

## [1.9.0] - 2026-06-17 — RusshClient + known_hosts + MockSshServer

### Added
- `m5tui-ssh::russh_client::RusshClient` — real `russh`-backed
  `SshClient` implementation. Implements the full trait
  surface (connect, exec, pty, scp_upload, scp_download,
  disconnect), mapping internal `RusshError` to the public
  `SshError`.
- `russh_client::Endpoint::from_profile` — host/port parser with
  default port 22.
- `russh_client::verify_known_host` — pure known_hosts checker
  with hex-prefix matching. Returns `Known` / `Unknown` /
  `HostKeyRejected`.
- `russh_client::known_hosts_entry` — builds a known_hosts line
  from host/port/keytype/key bytes.
- `russh_client::MockSshServer` — in-process server stub for
  tests; records connect attempts, learned entries, advertises a
  deterministic 8-byte key prefix.

## [1.8.0] - 2026-06-17 — xtensa CI + build script + justfile

### Added
- `scripts/build-esp.sh` — bash build/flash/monitor pipeline for the
  Cardputer-Adv. Sources espup's `export-esp.sh`, installs the
  `xtensa-esp32s3-espidf` target if missing, runs `cargo check` and
  optionally `cargo build`, then calls `espflash` for the requested
  port. `--debug` and `--release` profiles; `--flash PORT` and
  `--monitor PORT` flags.
- `justfile` — task runner for the host gates, sim run, device
  build, flash, flash-and-monitor, push, and release tag. `just`
  without arguments lists the recipes.
- `.github/workflows/xtensa.yml` is now a real cross-compile job
  that installs espup, libclang, the xtensa-esp32s3-espidf target,
  and runs `cargo check -p m5tui-bin --target xtensa-esp32s3-espidf`
  in addition to the `device` feature build (which is allowed to
  fail with a clear warning until M5GFX bindings are added).

## [1.7.0] - 2026-06-17 — OMP exec transport + MockOmpServer

### Added
- `m5tui-omp::OmpTransport` (Pty / Exec / WebSocket variants) on the
  `OmpSession` trait, with `transport()` and `attach_exec()` methods.
- `StubOmpSession` now tracks the active transport and defaults to
  `Pty` on `start()`.
- `MockOmpServer` — in-process OMP RPC server for tests, with a
  `round_trip_line()` convenience helper. 8 new tests.

## [1.6.0] - 2026-06-17 — Market live preview summary

### Added
- `m5tui-market::preview_summary(theme) -> [String; 4]` — 4-line,
  40-column ASCII summary of a previewed theme for the catalog picker
  overlay. Always 4 lines, never wider than 40 columns. 1 new test.

## [1.5.0] - 2026-06-17 — Book author mode + CHANGELOG/ROADMAP sync

### Added
- `m5tui-book::Spell::render_prompts()` with required/default markers,
  `validate_values`, `values_with_defaults`, and `AuthorEditor`
  fluent builder. 5 new tests.
- `CHANGELOG.md` and `ROADMAP.md` updated to reflect v1.1–v1.4
  status.

## [1.4.0] - 2026-06-17 — Theme draft save + doctor report + VU meter

### Added
- `m5tui-status` crate (new lib content): `BatteryStatus`, `WifiStatus`,
  `TailscaleStatus`, `ServerHealth`, `OmpPing`, `DiskUsage`, `Uptime`
  readings with healthy/warn/fail thresholds. `DoctorSnapshot::render_markdown()`
  produces a self-contained doctor report; `report_filename()` names
  the file. 12 new tests.
- `m5tui-handoff`: `parse_hit_jsonl` / `parse_hits_jsonl` for
  `obsidian-memory` output. `Hit::display_line()` formats a single
  40-col line. `SearchHistory` with `to_jsonl` / `load_jsonl`
  round-trip and silent-drop on corrupt lines. 8 new tests.
- `m5tui-voice`: `PlaybackRequest` (once / looped / at tick) and
  `BootSound` (Boot / Click / Arp / Silent) with asset path and
  fallback frequency. 2 new tests.
- `m5tui-core`: `widgets/vu.rs` — VU meter (10-cell bar + peak marker)
  for the cockpit bottom bar. `bar_string()` is the pure formatter;
  `draw()` mutates a `Frame` using `theme.palette.{ok,accent,err,dim}`.
  9 new tests.
- `m5tui-core`: `KeyAction::ForkDraftTheme` / `CommitDraftTheme` /
  `DiscardDraftTheme` wire the theme editor to the persist layer via
  `Outgoing::SaveTheme(Box<Theme>)`. `AppState` gains `theme_draft`,
  `theme_menu_index`, `theme_draft_palette` fields. 4 new reducer
  tests.

## [1.3.0] - 2026-06-17 — M3-M6 modal overlays wired to keymap + reducer

### Added
- 9 new `KeyAction` variants: `OpenProfilePicker`, `OpenBook`, `OpenVoice`, `OpenFirstBoot`, `OpenSettings`, `RunDoctor`, `OpenHandoff`, `OpenMemory`, `SaveMemo`.
- 8 matching `Outgoing` side-effect variants plus `PickProfile` and `RunSpell` for picker results.
- 8 new `Mode` variants and matching themed placeholder renderers in `widgets/overlay.rs` (profile picker, book, voice, first-boot wizard, settings, doctor, handoff, memory).
- `keymap::parse_chord()` binds `;p` profile picker, `;b` book, `;v` voice, `;n` first-boot, `;s` settings, `;D` doctor, `;h` handoff, `;m` memory, `;w` save memo.

## [1.2.0] - 2026-06-17 — OMP JSON codec, handoff ;continue

### Added
- `m5tui-omp`: `JsonCodec` (serde_json, kind/body wire format) round-trippable for all 9 frame kinds.
- `m5tui-omp`: `session_create`, `session_checkpoint`, `session_switch` frame builders.
- `m5tui-handoff`: `continue_prompt` + `continue_frame` helpers wire handoff metadata into an OMP `session.create` frame with `system_prefix` and `project`.

## [1.1.0] - 2026-06-17 — Stub-to-real wave 1

### Added
- `m5tui-persist`: `Driver` trait + `FsDriver` + `MemoryDriver`; atomic write via temp+rename; JSONL append with 512KB rotation; versioned `Migration` runner.
- `m5tui-profile`: YAML loader via `serde_yaml`; `ProfileRegistry` trait; `FileRegistry` mtime hot-reload; `InMemoryRegistry`; `resolve_proxy_jump` chain.
- `m5tui-market`: `ureq` HTTPS fetch; JSON catalog parser; offline cache at `market/catalog.json`; install + publish.
- `m5tui-voice`: `cpal` `AudioIn`/`AudioOut` under `host-audio` feature; WAV encoder/decoder; PTT state machine; `VoiceInbox`.
- `m5tui-device`: trait surface for `Display`/`Keyboard`/`Imu`/`AudioIn`/`AudioOut`/`Storage` + `HostDevice` implementation; `device` feature flag reserved for `esp-idf`.

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
[0.2.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v0.2.0
[1.0.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.0.0
[1.1.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.1.0
[1.2.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.2.0
[1.3.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.3.0
[1.4.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.4.0
[1.5.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.5.0
[1.6.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.6.0
[1.7.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.7.0
[1.8.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.8.0
[1.9.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.9.0
[1.10.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.10.0
[1.11.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.11.0
[1.12.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.12.0
[1.13.0]: https://github.com/NaustudentX18/m5tui/releases/tag/v1.13.0
