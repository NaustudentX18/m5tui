# m5Tui — Remaining Work Plan

> Captured: 2026-06-17
> Current baseline: `c730af8` — v1.0.0 structurally shipped, `cargo test --workspace` = 213 passed.
> Location: `/home/pi/m5Tui/`

## 1. Current State at a Glance

### ✅ Done (genuinely implemented + tested)

- **M0 Foundation:** Cargo workspace, `m5tui-core` pure state/reducer/framebuffer, sim RGBA8888 backend, host binary, CI matrix, tests.
- **M1 Cockpit shell:** `cockpit`, `palette`, `help`, `toast` widgets; 56-key Cardputer keymap; fuzzy matcher; 12 built-in palette commands; event + side-channel reducer (`step`).
- **M2 Theme engine:** `m5tui-themes` crate, YAML parser/validator, 6 built-in themes, theme-aware `render`, `theme_editor` overlay (view-only), sim golden tests.
- **M3–M6 + v1.0 structural wiring:** all crates exist, compile, have trait stubs and unit tests; `m5tui-bin/tests/integration_v1.rs` exercises every public API end-to-end in-memory.

### ⚠️ Stubbed / Not Real

- `m5tui-device`: only host stub display + RGB565 conversion. No ESP32-S3 / M5GFX / ST7789 driver, no I2C keyboard/IMU/audio, no SD layer.
- `m5tui-ssh`: `StubSshClient`; no real `russh` connection, PTY, keepalive, `known_hosts`, SCP.
- `m5tui-profile`: in-memory registry only; no YAML loader, hot-reload, jump host, first-boot wizard.
- `m5tui-persist`: `MemoryDriver` only; no SD path layout, atomic writes, migrations, JSONL logs.
- `m5tui-omp`: `StubOmpSession`; no JSON-RPC codec, no real `omp --mode rpc` pipe, no widgets for tool_call/todo_update/subagent.
- `m5tui-voice`: WAV codec + state machine; no real `AudioIn`/`AudioOut` (cpal or esp-i2s), no PTT hardware integration.
- `m5tui-book`: in-memory registry; `book/*.yaml` exist but are not loaded at runtime, no on-device author mode.
- `m5tui-market`: stub client only; no HTTPS client (ureq), no real catalog fetch/install/publish, no offline cache implementation.
- `m5tui-handoff`: in-memory store + markdown renderer stub; no SFTP fetch, no vault search integration.
- `m5tui-status` / `m5tui-doctor`: crate does not exist yet.
- `m5launcher`: `app.json` template only; no M5Stack M5Launcher metadata, icon, or device build.

---

## 2. Detailed Remaining TODO

### Workstream A — Device Target (Unblocks Everything Real)

1. **Set up ESP32-S3 toolchain on the build host.**
   - Install `espup` or ESP-IDF + `xtensa-esp32s3-elf-gcc`.
   - Add `xtensa-esp32s3-espidf` target to Rust.
   - Verify `cargo build --target xtensa-esp32s3-espidf --release` from a hello-world project first.
   - *Risk:* 1.5–2 GB download; may want to do it on CI or a faster machine and copy artifacts.

2. **Implement `m5tui-device` real drivers.**
   - Add `esp-idf-sys`/`esp-idf-hal` dependency guarded by a `device` feature.
   - Implement `M5Display` for ST7789V2 via M5GFX or `esp-idf-hal` SPI: init, `push_image`, brightness/backlight PWM.
   - Implement `cardputer_kb` hardware driver: read TCA8418 key matrix, emit `m5tui_core::Event::Key`.
   - Implement IMU wake/shake via BMI270.
   - Implement SD card SPI (CS G12) driver for persistence.
   - Implement ES8311 + I2S `AudioIn`/`AudioOut` for `m5tui-voice`.
   - Add a device `main.rs` or feature-gated entry that owns the real event loops.

3. **Create a device build + flash pipeline.**
   - Add `m5launcher/icon.png`.
   - Add a `build-esp.sh` or `justfile` recipe for release build + `esptool.py` flash.
   - Wire `.github/workflows/xtensa.yml` to do real cross-compiles (drop the stub guard).
   - Decide binary distribution: GitHub Releases + M5Launcher manifest.

### Workstream B — SSH + Profiles (M3)

4. **Replace `StubSshClient` with real `russh` client.**
   - Add `russh` + `russh-keys` deps.
   - Implement `client.rs`: connect, auth via key/agent/password, disconnect.
   - Implement `pty.rs`: channel open, resize, read loop feeding `Event::SshBytes`.
   - Implement `keepalive.rs`: ping every 15s, 3 misses → reconnect with backoff.
   - Implement `known_hosts.rs`: SD-backed known_hosts file.
   - Implement `scp.rs`: push WAV to `~/voice-inbox/`.

5. **Implement `m5tui-profile` registry.**
   - YAML loader for `profiles/*.yaml`.
   - Hot-reload on mtime change.
   - Default profile selection.
   - Jump host / `ProxyJump` resolution.
   - Profile picker (`;p`) wired in `cockpit` + keymap.

6. **Implement `m5tui-persist` SD layer.**
   - SD layout: `/sd/m5tui/themes/`, `/sd/m5tui/profiles/`, `/sd/m5tui/voice/`, `/sd/m5tui/history/`, `/sd/m5tui/memos/`, `/sd/m5tui/logs/`.
   - Atomic write helper (`write-temp + rename`, fsync).
   - JSONL append-only log rotation at 512 KB.
   - Schema migration runner for persisted config files.

7. **First-boot wizard.**
   - 6-screen flow: Wi-Fi scan/PSK, Tailscale auth, SSH key generation, server pick, theme pick, mic test.
   - Persist resulting profile/config to SD.
   - Add `;n` keybinding.

### Workstream C — OMP Integration (M4)

8. **Implement `m5tui-omp` real RPC.**
   - JSON-RPC 2.0 line codec over SSH exec channel.
   - Frame parser for: `agent.text_delta`, `tool_call.start/end`, `todo_update`, `subagent`, `error`, `session.usage`, `session.thinking`.
   - `OmpSession` start/stop/switch (`;a`).
   - Add core widgets for tool calls, todo list, subagent tree, thinking badge.

9. **Profile-aware OMP session lifecycle.**
   - Start `omp --mode rpc` in exec channel when profile connects.
   - Send `session.create` with model from profile.
   - Pipe `session.checkpoint` on `;a` switch.

### Workstream D — Voice / PTT (M5)

10. **Real audio I/O.**
    - Host: cpal `AudioIn`/`AudioOut` for Pi dev/testing.
    - Device: esp-i2s + ES8311 codec config (I2C G8/G9, I2S bus).
    - Implement chunked WAV encoder during recording.
    - VU meter overlay in `cockpit` bottom bar.

11. **PTT integration.**
    - `;v` hold-to-record / tap-for-menu.
    - Auto-push to `~/voice-inbox/` when `voice.auto_push: true`.
    - Optional `advdeck-bridge plan` enqueue when `voice.auto_enqueue_plan: true`.
    - Playback via `;p` / `;voice-list`.

### Workstream E — Book of Commands (M5a)

12. **Load real YAML spells.**
    - `m5tui-book::Registry` walks `book/*.yaml`, validates, hot-reloads.
    - Parse the 7 sample spells already in `book/`.
    - Wire `;b` to the book UI.

13. **Spell execution.**
    - Prompt rendering, template expansion, undo stack.
    - Author mode (`;b L`) to create/edit spells on-device.
    - Export/import to SD.

### Workstream F — Community Market (M5b)

14. **Real market client.**
    - Add `ureq` (or `reqwest` if async) for HTTPS catalog fetch.
    - Implement catalog parser, preview, install, publish.
    - Offline cache at `/sd/m5tui/market/catalog.json`.
    - Live preview pane rendering cockpit with the preview theme.

15. **Backend.**
    - Set up GitHub Pages catalog + B2/R2 asset bucket.
    - Publish flow: catalog PR + asset upload.

### Workstream G — Handoff + Vault (M6)

16. **Handoff viewer.**
    - List projects with `agent-prompt.md` via SFTP over the SSH connection.
    - Render Markdown inline (no external lib, ~400 lines).
    - `;continue` starts OMP session with prompt as system prefix.
    - `;save` to `/sd/m5tui/memos/<date>.md`.

17. **Vault search.**
    - `;m` runs `obsidian-memory search` over SSH.
    - Render JSONL hits.
    - Persist search history.

### Workstream H — Status + Doctor (M7)

18. **Create `m5tui-status` crate.**
    - Battery: AXP2101 I2C poll.
    - Wi-Fi: RSSI from esp-wifi.
    - Tailscale: parse `tailscale status` or daemon socket.
    - Server health: keepalive-driven status.
    - Uptime/disk/OMP ping polled over SSH.

19. **Implement `;D` doctor.**
    - Full self-check scoreboard.
    - Export report to `/sd/m5tui/logs/doctor-<ts>.md`.

### Workstream I — Boot + Polish

20. **Boot screen.**
    - ASCII logo, fake loading animation, skippable with `enter`.
    - Sound: boot/click/arp WAV playback if theme has sound enabled.

21. **Theme editor save/export.**
    - Make the editor actually mutate the draft theme.
    - Save to `/sd/m5tui/themes/<name>.yaml`.
    - Export YAML to stdout.

22. **Settings UI.** ✅ DONE v1.14.0 — interactive overlay with `>` cursor marker, j/k cursor nav, `-`/`+` brightness adjust, space/enter to toggle sound / cycle IMU wake (`Off`/`Shake`/`Tilt`). Read-only rows (wifi SSID, tailscale status) displayed from `AppState`. Renderer: `crates/m5tui-core/src/widgets/overlay.rs::render_settings`. Reducer arms: `SettingsUp/Down/Left/Right/Toggle`.

23. **About + log viewer.**
    - `;?` from help opens about screen.
    - `;L` log viewer with pager.

### Workstream J — CI / Release / Docs

24. **GitHub repo + release hygiene.**
    - Push to `NaustudentX18/m5tui` if not already done.
    - Tag `v1.0.0` and confirm CI green on first push.
    - Move xtensa CI from stub to real cross-compile.

25. **Update docs to reflect real vs stubbed.**
    - Add "What's not done yet" section to `README.md`.
    - Update `CHANGELOG.md` with `Unreleased` entries as work lands.
    - Keep `ROADMAP.md` honest: M3–M6 are structurally done but functionally stubbed.

---

## 3. Suggested Order of Attack

1. **Repo push + CI baseline** (0.5 day) — unblock public collaboration and real CI.
2. **ESP32 toolchain + device crate real drivers** (1–2 weeks) — this is the critical path; nothing else matters on-device without it.
3. **Persist + Profiles + SSH** (1–2 weeks) — once device can talk SD and network, wire real connection logic.
4. **OMP real integration** (1 week) — voice and widgets depend on a live session.
5. **Voice real audio** (1 week) — cpal on host first, then esp-i2s on device.
6. **Book, Market, Handoff** (1.5 weeks) — parallel once SSH/OMP exist.
7. **Status + Doctor + Boot polish** (1 week) — final UX layer.

---

## 4. Immediate Next Action

> **Decide whether to (a) install the ESP toolchain locally and start `m5tui-device`, or (b) push the repo to GitHub first and let CI validate the cross-compile.**
>
> Recommendation: do (b) first — it's 30 minutes and gives a public baseline. Then do (a) in parallel on a faster host if available.

