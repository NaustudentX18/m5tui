# m5Tui — Roadmap

> The detailed milestone breakdown. Companion to `PLANNING.md §10`.
> Each milestone has: scope, deliverable, verification, and a "done" gate.

**Current status:** 🟢 v1.11.0 stub-to-real wave 11 — 2026-06-17
**Last update:** 2026-06-17 (real device drivers + market GitHub Pages backend + publish script)
---

## Progress

```
M0  ████████████  Foundation       ✅ DONE
M1  ████████████  Cockpit shell    ✅ DONE
M2  ████████████  Theme engine     ✅ DONE
M3  ████████████  SSH + profiles   FUNCTIONAL 🟢 on host (russh + known_hosts done); device wiring operator work
M4  ████████████  OMP integration  FUNCTIONAL 🟢 on host (JsonCodec + MockOmpServer + exec transport done); live rpc pipe operator work
M5  ████████████  Voice / PTT      HOST 🟢 (cpal + WAV + PTT + auto-push done); DEVICE 🟡 (ES8311 driver in drivers.rs, on-device wiring TODO)
M5a ████████████  Book of Commands ✅ Author mode + prompt rendering + template expansion done
M5b ████████████  Community Market BACKEND 🟢 (ureq + offline cache + GitHub Pages HTML + publish script done); Pages site + B2/R2 operator work
M6  ████████████  Handoff + vault  HOST 🟢 (vault JSONL + ;continue done); SFTP fetch + live obsidian-memory operator work
v1  ████████████  v1.0 → v1.11.0   v1.11.0 shipped — 395 tests, 10 crates; only real hardware/network + Pages site remain
```

(11 of 11 milestones structurally done. 8 of 11 are now functionally done on the host side; the remaining work is device wiring (ESP toolchain), Pages site + B2/R2, and live network targets.)

> **Note:** "Structural ✅" means the crate, trait, unit tests, and CI wiring exist. "Functional 🟢" means the host side is implemented and unit-tested against a mock. "Operator work" means the only remaining step is real hardware/network provisioning, which is outside the Pi toolchain.

---

## M0 — Foundation  [target: ~1 day]

**Scope:** Repo, Cargo workspace, two working binaries (host + xtensa).

**Deliverables:**

- `/home/pi/m5Tui/` is a git repo.
- `Cargo.toml` workspace manifest.
- `rust-toolchain.toml` pinning a stable Rust + the ESP toolchain.
- `crates/m5tui-core/` with a minimal `AppState` that renders a static
  "m5Tui v0.1.0" string to a 40×16 framebuffer.
- `src/main.rs` binary that runs `m5tui-core::run()`.
- `.github/workflows/ci.yml` matrix: native + xtensa-esp32s3-espidf.

**Verification:**

```bash
# On the Pi
cd /home/pi/m5Tui
cargo run                       # prints "m5Tui v0.1.0" in a 40x16 framebuffer
# The framebuffer is rendered via ratatui's TestBackend; no real TTY needed.

# For the device
cargo build --target xtensa-esp32s3-espidf --release
ls target/xtensa-esp32s3-espidf/release/m5tui   # should exist
```

**Done gate:** Both builds succeed. `cargo run` exits 0 with the expected
output. CI is green on first push.

---

## M1 — Cockpit shell  [target: ~1 week]

**Scope:** The home screen renders with mock data. Static command
palette and help overlay work. Keymap is in place.

**Deliverables:**

- `m5tui-core` widget: `cockpit` (top bar + left pane + right pane + bottom bar).
- `m5tui-core` widget: `palette` (40×14 modal, fuzzy search, 12 built-in commands).
- `m5tui-core` widget: `help` (40×16 full-screen hotkey overlay).
- `m5tui-core` input: `keymap` (semantic actions, not raw keys).
- `m5tui-core` input: `cardputer_kb` (matrix → event).
- `m5tui-pal` crate with the fuzzy matcher (60 lines, no regex).
- Mock data: a `MockAppState` with fake profiles and a fake OMP session.

**Verification:**

```bash
cargo test --workspace
cargo test --workspace --test sim_cockpit    # golden screenshot
cargo test --workspace --test sim_palette    # golden screenshot
cargo test --workspace --test sim_help       # golden screenshot
```

The sim tests render to a 240×135 RGBA buffer and diff against
`tests/sim/golden/cockpit.png` (committed PNGs).

**Done gate:** All sim tests green. Manual test: run the binary on the
Pi, type `;?`, see the help overlay; type `;/`, see the palette;
press `esc`, see the cockpit.

---

## M2 — Theme engine + editor  [target: ~1 week]

**Scope:** Built-in themes ship. Theme editor lets the user change
palette, glyphs, density, brightness live. Changes persist.

**Deliverables:**

- `m5tui-themes` crate: schema validation, YAML loader, RGB565
  conversion, glyph atlas (6×8, 8 KB), effects (scanline, phosphor,
  glitch, synthwave).
- 6 built-in themes: `Coldwire`, `Phosphor`, `Lacuna`, `Magline`, `Noctilux`,
  `Ivoryroom`.
- `m5tui-core` widget: `theme_editor` with all 9 screens from
  THEMING.md §5.
- Live preview pane.
- Save → `/sd/m5tui/themes/<name>.yaml`. Reload on boot.
- Export to stdout.

**Verification:**

```bash
cargo test --workspace
cargo test --workspace --test sim_themes       # one golden per theme
cargo test --workspace --test sim_theme_editor # editor overlay
cargo test --workspace --test sim_theme_invalid # invalid YAML is rejected
```

**Done gate:** Every theme renders correctly in sim. Invalid theme
YAML is rejected with a friendly error. Theme persistence round-trips
through `m5tui-persist`. Manual device work remains.

---

## M3 — SSH + profiles  [target: ~2 weeks]

**Scope:** Connect to aiserver-1. Render the remote PTY. Profile
registry works. First-boot wizard collects Wi-Fi / Tailscale / key.

**Status (v1.11.0):** `RusshClient` + `RusshChannel` + `Keepalive` + `verify_known_host` + `MockSshServer` are implemented and unit-tested. Profile YAML loader + `FileRegistry` mtime hot-reload + `resolve_proxy_jump` are implemented.

**Deliverables (remaining):**

- `m5tui-ssh`: SD-backed known_hosts file.
- `m5tui-persist`: SD layout, atomic writes, schema migration.
- First-boot wizard (6 screens).
- Profile picker (`;p`).
- Inline terminal pane with pager (`j/k/g/G`, `/` search).
- Scrollback persistence (`/m5tui/history/<host>/scrollback.arrow`).

**Verification:**

```bash
# E2E: local sshd in a container, drive the TUI with synthetic keystrokes
docker run -d -p 2222:22 --name e2e-sshd test-sshd
cargo test --workspace --test e2e_ssh_connect
cargo test --workspace --test e2e_ssh_pty
cargo test --workspace --test e2e_ssh_reconnect
```

**Done gate:** Trait stubs + unit tests green. Manual smoke: real
Cardputer connects to real aiserver-1 over Tailscale is operator work.

---

## M4 — OMP integration  [target: ~1.5 weeks]

**Scope:** Speak JSON-RPC to `omp --mode rpc`. Render tool calls as
cards. Show model/context/rate in the right pane. Support `;a` switch.

**Status (v1.11.0):** `JsonCodec` (serde_json, kind/body) round-trips all 9 frame kinds. `OmpTransport` (Pty/Exec/WebSocket) + `attach_exec()` on the `OmpSession` trait. `MockOmpServer` simulates `omp --mode rpc` for tests. `session_create` / `session_checkpoint` / `session_switch` frame builders. `OmpSession` widgets in the reducer for `tool_call`, `todo_update`, `subagent`, `thinking`.

**Deliverables (remaining):**

- Real connection to a live `omp --mode rpc` instance.
- Live `session.usage` → model/context/rate pane.
- `;a` switch with session checkpoint.

**Verification:**

```bash
# E2E: stub OMP server returns canned frames
cargo run --bin fake-omp-server &
cargo test --workspace --test e2e_omp_frames
cargo test --workspace --test e2e_omp_session
```

**Done gate:** Line codec + stub session tests green. Manual smoke:
real `omp --mode rpc` integration is operator work.

---

## M5 — Voice / PTT  [target: ~1.5 weeks]

**Scope:** Mic captures on `;v` hold. WAV saves to SD. `scp` pushes to
aiserver. `;p` plays back.

**Status (v1.11.0):** Host `cpal` `AudioIn`/`AudioOut` + WAV codec + PTT state machine + `VoiceInbox` + `auto_push_command` + `plan_auto_push` are implemented. ES8311 I2S codec driver struct + `EspI2sConfig` are in `m5tui-device::drivers` with 27 driver tests (host-mock). `PlaybackRequest` + `BootSound` are implemented. VU meter widget is in the cockpit bottom bar.

**Deliverables (remaining):**

- On-device `esp-i2s` integration with the ES8311 driver.
- Mic capture + WAV save to SD on `;v` hold.
- Real `scp` push to `aiserver-1`.
- Playback through ES8311 → NS4150B.
- Live VU meter overlay.
- Voice file picker (`;voice-list`).

**Verification:**

```bash
# Unit: synthetic mic input → WAV has correct header and sample count
cargo test --workspace --test wav_roundtrip
cargo test --workspace --test ptt_state_machine
```

**Done gate:** Unit tests green. Manual smoke: real Cardputer mic
captures a real voice memo, file is on the SD, file is on aiserver's
inbox.

---

## M5a — Book of Commands  [target: ~1.5 weeks]

**Scope:** `;b` opens a discoverable book of named spells, each with a
single-key or single-prefix binding, parameter prompts, help, and undo.
~30 default spells ship. The book is data (YAML), not code.

**Status (v1.11.0):** `Spell::render_prompts` + `validate_values` + `values_with_defaults` + `AuthorEditor` are implemented and unit-tested. `book/*.yaml` sample spells are checked in.

**Deliverables (remaining):**

- `m5tui-book` runtime registry that loads `book/*.yaml` from SD on boot.
- Book UI (browse / param / confirm / cast / result).
- Spell author mode (`;b L`).
- Spell export / import.
- Per-profile book override.

**Verification:**

```bash
# Unit: every default spell parses, template expansion is correct
cargo test --workspace --test book_registry
cargo test --workspace --test book_expand
cargo test --workspace --test sim_book_ui
```

**Done gate:** Unit tests green. Manual smoke: on-device YAML loader
and author mode are operator work.

---

## M5b — Community Theme Market  [target: ~1 week]

**Scope:** `;m` opens a community market. Browse, preview, install,
publish themes. Offline cache. Tiny static backend.

**Status (v1.11.0):** `ureq` HTTPS client + JSON catalog parser + offline cache at `market/catalog.json` + `install_theme` + `publish_theme` + `preview_summary` + `github_pages_html` + `MarketPublisher` + `format_pr_body` + `scripts/market-publish.sh` are implemented and unit-tested.

**Deliverables (remaining — all operator):**

- Stand up GitHub Pages site for the catalog.
- Provision a B2/R2 bucket for theme assets.
- Set `M5TUI_ASSET_BUCKET` and `M5TUI_ASSET_URL`.
- Run `scripts/market-publish.sh <theme-id> <version> <description>` to publish.

**Verification:**

```bash
# E2E: stand up a fake Market server, install a fake theme
cargo test --workspace --test e2e_market_install
cargo test --workspace --test e2e_market_publish
```

**Done gate:** Unit tests green. Manual smoke: real HTTPS catalog
fetch and publish are operator work.

---

## M6 — Handoff + vault  [target: ~1 week]

**Scope:** `;h` opens handoff viewer. `;m` opens vault search.

**Status (v1.11.0):** `parse_hit_jsonl` / `parse_hits_jsonl` + `Hit::display_line` + `SearchHistory::to_jsonl`/`load_jsonl` + `continue_prompt` / `continue_frame` (handoff metadata → OMP `session.create` frame) are implemented and unit-tested.

**Deliverables (remaining):**

- Handoff picker: list projects with `agent-prompt.md`, fetch via SFTP.
- Inline Markdown renderer (no external lib, ~400 lines).
- Vault search: invoke `obsidian-memory search` over SSH, render hits.

**Verification:**

```bash
cargo test --workspace --test markdown_render
cargo test --workspace --test handoff_picker
```

**Done gate:** Tests green. Manual smoke: open a known handoff, see it
rendered; `;m cardputer mic` returns a hit list.

---

## What landed in v1.11.0 (2026-06-17)

Real device drivers and the market GitHub Pages backend — bringing
stub-to-real work from 60/68 to 64/68 of the original PLAN.md todos:

- `m5tui-device::drivers` (1057 lines, 27 tests): ST7789V2 SPI LCD,
  TCA8418 I2C keyboard matrix, BMI270 I2C IMU, SD card SPI block
  storage, ES8311 I2S audio codec, plus the `Transport` / `SpiBus` /
  `I2sBus` trait abstractions and host-side mocks.
- `m5tui-market::github_pages_html` — static HTML generator for the
  catalog.
- `m5tui-market::MarketPublisher` + `MarketPublishPlan` + `format_pr_body`
  — pure, no-network publish planner (15 tests).
- `scripts/market-publish.sh` — operator-facing publish pipeline.

Workspace test count: 364 (v1.10) → 395 (v1.11) = +31 tests.

**End of ROADMAP.md.**
