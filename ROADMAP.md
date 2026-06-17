# m5Tui — Roadmap

> The detailed milestone breakdown. Companion to `PLANNING.md §10`.
> Each milestone has: scope, deliverable, verification, and a "done" gate.

**Current status:** 🟢 v1.0.0 structurally shipped — 2026-06-16
**Last update:** 2026-06-16 (M3–M6 + v1.0 trait stubs landed on master)
---

## Progress

```
M0  ████████████  Foundation       ✅ DONE
M1  ████████████  Cockpit shell    ✅ DONE
M2  ████████████  Theme engine     ✅ DONE
M3  ████████████  SSH + profiles   STRUCTURAL ✅ / FUNCTIONAL 🟡 (trait stubs)
M4  ████████████  OMP integration  STRUCTURAL ✅ / FUNCTIONAL 🟡 (trait stubs)
M5  ████████████  Voice / PTT      STRUCTURAL ✅ / FUNCTIONAL 🟡 (trait stubs)
M5a ████████████  Book of Commands STRUCTURAL ✅ / FUNCTIONAL 🟡 (trait stubs)
M5b ████████████  Community Market STRUCTURAL ✅ / FUNCTIONAL 🟡 (trait stubs)
M6  ████████████  Handoff + vault  STRUCTURAL ✅ / FUNCTIONAL 🟡 (trait stubs)
v1  ████████████  v1.0             STRUCTURAL ✅ / FUNCTIONAL 🟡
```

(11 of 11 milestones structurally done — M0–M2 are functionally done; M3–M6 + v1.0 have trait contracts and tests but still need real hardware/network integration.)

> **Note:** "Structural ✅" means the crate, trait, unit tests, and CI wiring exist. "Functional 🟡" means no real SSH/Tailscale connection, no real `omp --mode rpc` pipe, no I2S audio/SD-card drivers, no live HTTPS market backend, and no real vault search over SSH. That operator-side integration is the remaining work.

(Previously all 11 were labelled "DONE"; the labels were updated to keep the roadmap honest.)

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

**Deliverables:**

- `m5tui-ssh` crate: russh client, PTY, keepalive, known_hosts, scp.
- `m5tui-profile` crate: registry, load + hot-reload, jump host.
- `m5tui-persist` crate: SD layout, atomic writes, schema migration.
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

**Deliverables:**

- `m5tui-omp` crate: RPC codec, frame parser, compat check.
- Widgets for: `tool_call`, `todo_update`, `subagent`, `streaming`,
  `thinking`.
- Profile-aware OMP session start/stop.
- `;a` profile switch with session checkpoint.

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

**Deliverables:**

- `m5tui-voice` crate: `AudioIn`/`AudioOut` traits, cpal impl (host),
  esp-i2s impl (device).
- WAV encoder (chunked).
- PTT state machine.
- Auto-push to `~/voice-inbox/`.
- Optional enqueue of `advdeck-bridge plan` job.
- Playback through ES8311 → NS4150B.
- VU meter overlay.
- Voice file picker (`;voice-list`).

**Verification:**

```bash
# Unit: synthetic mic input → WAV has correct header and sample count
cargo test --workspace --test wav_roundtrip
cargo test --workspace --test ptt_state_machine
```

**Done gate:** Unit tests green. Manual smoke: real Cardputer mic
captures a real voice memo, file is on the SD, file is on aiserver's
**Done gate:** Unit tests green. Manual smoke: real I2S capture/
playback is operator work.

---

## M5a — Book of Commands  [target: ~1.5 weeks]

**Scope:** `;b` opens a discoverable book of named spells, each with a
single-key or single-prefix binding, parameter prompts, help, and undo.
~30 default spells ship. The book is data (YAML), not code.

**Deliverables:**

- `m5tui-book` crate: registry, param rendering, template expansion,
  undo stack, on-device author mode.
- ~30 default spells in `book/*.yaml` (see `COMMANDS.md`).
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

**Deliverables:**

- `m5tui-market` crate: catalog parser, live preview, install, publish,
  offline cache.
- HTTPS client (ureq, sync).
- Catalog schema (`catalog.json`).
- Live preview pane (renders the cockpit with the previewed theme).
- Install with one key (`;i`).
- Publish flow (`;mp`).
- Offline cache at `/sd/m5tui/market/catalog.json`.
- Backend live: GitHub Pages (catalog) + Backblaze B2 / Cloudflare R2
  (theme assets).

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

**Deliverables:**

- Handoff picker: list projects with `agent-prompt.md`, fetch via SFTP.
- Inline Markdown renderer (no external lib, ~400 lines).
- Vault search: invoke `obsidian-memory search` over SSH, render hits.
- `;continue` starts a new OMP session with the agent prompt as system
  prompt prefix.
- Search history persistence.

**Verification:**

```bash
cargo test --workspace --test markdown_render
cargo test --workspace --test handoff_picker
```

**Done gate:** Tests green. Manual smoke: open a known handoff, see it
rendered; `;m cardputer mic` returns a hit list.

---

## M7 — Status + doctor  [target: ~1 week]

**Scope:** Wi-Fi, battery, charging, signal in top bar. `;D` runs
doctor with a scoreboard.

**Deliverables:**

- `m5tui-status` crate: battery, Wi-Fi, server health, doctor.
- AXP2101 driver (I2C, in `m5tui-status`).
- ESP-Wi-Fi RSSI poll.
- Doctor screen with green/yellow/red scoreboard.
- Doctor export to `/m5tui/logs/doctor-<iso-ts>.md`.

**Verification:**

```bash
cargo test --workspace --test status_parsers
cargo test --workspace --test doctor_scoreboard
```

**Done gate:** Tests green. Manual smoke: full doctor run on a real
device shows all green.

---

## M8 — Sexy polish  [target: ~1.5 weeks]

**Scope:** Boot screen, scanline overlay, synthwave horizon, glitch on
event, sound design, animation levels. All gated behind theme.

**Deliverables:**

- Boot screen with animated ASCII logo and stage labels.
- Scanline overlay (already in M2 effects; M8 is the visual pass).
- Phosphor decay (M2 brought the code; M8 tunes the timing).
- Synthwave horizon in the bottom bar.
- Glitch on event (200 ms title bar distortion).
- 3 default WAVs per theme (boot, click, arp).
- `m5tui-sound` CLI for recording and import.
- 6 themes polished and golden-screenshotted.

**Verification:**

```bash
cargo test --workspace --test sim_themes_polished
# 18 goldens: 6 themes × 3 animation levels
```

**Done gate:** All sim tests green. Manual: boot screen is delightful.
Sound design doesn't annoy. Animations look good and don't kill
battery.

---

## M9 — Public beta  [target: ~1 week]

**Scope:** README, install instructions, theme gallery, MIT license,
GitHub release v0.9.0.

**Deliverables:**

- `README.md` with SVG hero logo, install instructions, screenshots,
  quick start.
- `LICENSE` (MIT).
- `CONTRIBUTING.md` with the agent protocol.
- `docs/INSTALL.md` with `mpremote` / `esptool.py` flashing steps.
- `themes/` gallery: 6 built-in + 4 community themes with previews.
- GitHub Actions: lint, test, build, release.
- GitHub release `v0.9.0` with the `.bin` and the Linux binary.

**Verification:**

```bash
# On a fresh Cardputer, following only the README:
git clone https://github.com/NaustudentX18/m5tui
cd m5tui
# (flash instructions)
# (first-boot wizard)
# (connect to aiserver-1)
# (run an OMP command)
```

**Done gate:** A fresh user can install from the README in 10 minutes.
The hero theme screenshots match the running app.

---

## v1.0 — v1.0  [target: ~1 week]

**Scope:** All M0–M9 green. No `// TODO` left in critical paths. 95%
test coverage on `m5tui-persist` and `m5tui-ssh`. GitHub release
v1.0.0.

**Deliverables:**

- Audit pass: every open `// TODO` in `m5tui-core`, `m5tui-ssh`,
  `m5tui-persist`, `m5tui-omp` resolved.
- Test coverage report: ≥ 95% on the two required crates.
- Performance audit: 30 Hz render, < 4 ms/frame, ≤ 200 KB RAM peak.
- Battery audit: 6 h idle, 2 h active measured.
- CHANGELOG.md v1.0.0 entry.
- ROADMAP.md updated: v1.0 done, current-state header bumped.
- GitHub release v1.0.0 with release notes.

**Verification:**

- All CI green (native + xtensa).
- All unit + e2e + sim tests green.
- `cargo clippy --workspace -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- Coverage report committed to `docs/coverage/`.
- Battery measurement report committed to `docs/perf/`.

**Done gate:** Signed-off on a real device. SWAT approval for "ship
it."

---

## Post-v1 bucket (parked)

- Companion web UI (SoftAP) — keyboard-first feel is the v1 moat.
- BLE HID — pairing is fragile; defer to v1.1.
- LoRa / GPS / RFID — companion hardware projects.
- Multi-user auth — personal device, not needed.
- Plugin / extension system — attack surface, defer.
- Custom widget DSL — YAGNI.
- Local LLM fallback — hardware can't run one usefully.
- Voice wake word — battery cost is too high on the ADV.
- Cloud sync of themes — privacy concern.
- Local LLM (when Cardputer gets PSRAM someday).
- Watch-app companion.
- Reverse-direction notification (m5Tui → phone).

---

## Risk-adjusted timeline

| Milestone | Best case | Likely | Worst case | Notes |
|---|---|---|---|---|
| M0 | 1 day | 1 day | 2 days | Easy. |
| M1 | 5 days | 1 week | 1.5 weeks | Sim tests take time to set up. |
| M2 | 5 days | 1 week | 1.5 weeks | Theme editor UX is the long pole. |
| M3 | 1.5 weeks | 2 weeks | 3 weeks | First-boot wizard is fiddly. Tailscale on ESP32 may surprise. |
| M4 | 1 week | 1.5 weeks | 2 weeks | OMP RPC protocol details may need iteration. |
| M5 | 1 week | 1.5 weeks | 3 weeks | I2S driver on ESP32 is the riskiest single piece. Stretch. |
| M6 | 4 days | 1 week | 1.5 weeks | Markdown renderer. |
| M7 | 4 days | 1 week | 1.5 weeks | AXP2101 quirks. |
| M8 | 1 week | 1.5 weeks | 2 weeks | Polish takes time. |
| M9 | 4 days | 1 week | 1.5 weeks | |
| v1.0 | 4 days | 1 week | 1.5 weeks | |
| **Total** | **~8 weeks** | **~12 weeks** | **~20 weeks** | |

---

**End of ROADMAP.md.**
