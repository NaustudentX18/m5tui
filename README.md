# m5Tui

> **m5Tui v1.11.0 — real device drivers + market GitHub Pages backend.** M0–M2 are functionally done; M3–M6 are now real (RusshClient, JsonCodec, host audio, live preview, vault JSONL) on the host side. M3/M4/M5/M5b still need the ESP toolchain + Pages site to talk to real hardware/network.

## What is m5Tui?

A sexy, keyboard-first, themable CLI/TUI that lives on the M5Stack
Cardputer-Adv and SSHs into `aiserver-1` (Pi 5) so you can run and
steer the agent ecosystem — exactly like Termius on your phone, except
it understands agents, voice, and the device.

```
+--------------------------------------------+
| aiserver-1  :: OK  :: up 4h12m  :: 73% bat |
+--------------------------------------------+
| AGENTS                | SESSION            |
| > aiserver-1/omp      | task 14/22  in_flt |
|   aiserver-1/bridge   | model MiniMax-M3  |
|   pc-ollama/midimax   | ctx 12K/512K       |
|   jbpi/pc-tailscale   | rate 31.4 t/s      |
+------------------------+-------------------+
| ;/ palette | ;v voice | ;t theme | ;? help |
+--------------------------------------------+
| > _                                                |
+--------------------------------------------+
```

## Status

🟢 **v1.11.0 — 2026-06-17** — Real device drivers (ST7789/TCA8418/BMI270/SD/ES8311, host-mock tested) + market GitHub Pages HTML backend + publish script. v1.10 added real russh client + channel + keepalive. 395 tests green across 10 crates. See `CHANGELOG.md` and `ROADMAP.md`.

## What's not done yet

M0–M2 are functionally done. M3–M6 + v1.x are now real on the host side; the only remaining work is operator-side integration:

- **M3 — SSH + profiles:** `RusshClient` / `RusshChannel` / `Keepalive` / `known_hosts` are implemented and unit-tested against a mock server. Real Tailscale connection to `aiserver-1`, first-boot wizard, jump host, and SD-backed known_hosts file are operator work.
- **M4 — OMP integration:** `JsonCodec` round-trips all 9 frame kinds; `MockOmpServer` simulates `omp --mode rpc`. Real connection to a live OMP instance is operator work.
- **M5 — Voice / PTT:** Host `cpal` audio I/O + WAV codec + PTT state machine + auto-push SCP planner are implemented and tested. Device `esp-i2s` + ES8311 codec wiring is the next wave.
- **M5a — Book of Commands:** `Spell::render_prompts` + `AuthorEditor` are implemented. Real on-device YAML loader, author mode, and SD-card storage are operator work.
- **M5b — Community Theme Market:** HTTPS `ureq` client + offline cache + `github_pages_html` + `MarketPublisher` + `scripts/market-publish.sh` are implemented. Operator needs to (a) stand up the GitHub Pages site, (b) provision a B2/R2 bucket, (c) set `M5TUI_ASSET_BUCKET` and `M5TUI_ASSET_URL`.
- **M6 — Handoff + vault:** `parse_hit_jsonl` / `SearchHistory` / `;continue` are implemented. Real SFTP fetch of `agent-prompt.md` + live `obsidian-memory search` invocation over SSH are operator work.
- **Device drivers:** `drivers.rs` defines ST7789V2 / TCA8418 / BMI270 / SD / ES8311 structs with `Transport` / `SpiBus` / `I2sBus` traits. Host-mock tests are green. The on-device `unsafe extern "C"` paths into `esp-idf-hal` are the final step; needs ESP-IDF + the Cardputer-Adv connected over USB.

All of the above is operator-side work. The Rust workspace, tests, CI, and trait contracts are ready for it.

M0 is "one fully done" per the user instruction: workspace scaffold, `m5tui-core` library, simulator backend that renders a 40x16 frame to a 240x135 RGBA buffer, unit + integration tests, CI matrix. The `m5Tui v0.1.0` title renders deterministically.

Round 1 SWAT decisions remain locked (Rust stack, device-native standalone v1, OMP 15.13.3 pinned, MIT, `NaustudentX18/m5tui`, voice in v1, Book of Commands required, Community Theme Market required, 6 pre-installed themes, `Coldwire` default, sound on in 4 of 6 themes). Read [`PLANNING.md`](./PLANNING.md) for the full SWAT brief.

## Documents in this folder

| File | Purpose |
|---|---|
| [`PLANNING.md`](./PLANNING.md) | **Start here.** The master SWAT brief: scope, architecture, features, theming, hardware, milestones, open questions. |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | The engineering spec: crate map (incl. `m5tui-book`, `m5tui-market`), transport, event flow, OMP RPC, voice subsystem, theming, persistence, failure modes, performance budget. |
| [`COMMANDS.md`](./COMMANDS.md) | The Book of Commands: discoverable, one-key, on-device library of named spells with parameters, help, and undo. |
| [`MARKET.md`](./MARKET.md) | The Community Theme Market: browse / preview / install / publish themes; tiny static backend. |
| [`FEATURES.md`](./FEATURES.md) | The complete feature inventory, milestone-tagged with P0/P1/P2 priorities. |
| [`THEMING.md`](./THEMING.md) | The theming system: design philosophy, the six pre-installed themes with descriptive names, schema, on-device editor, animations, sound. |
| [`HARDWARE.md`](./HARDWARE.md) | The Cardputer-Adv hardware reference: every pin, every bus, driver notes, power budget. |
| [`AGENTS.md`](./AGENTS.md) | How a coding agent should work on m5Tui. Hard rules, testing contract, definition of done. |
| [`ROADMAP.md`](./ROADMAP.md) | Milestone breakdown (M0–M9 + v1.0, with M5a Book and M5b Market) with verification criteria and risk-adjusted timeline. |
| [`PLAN.md`](./PLAN.md) | The detailed remaining-todo breakdown from the 2026-06-17 audit. |
| [`CHANGELOG.md`](./CHANGELOG.md) | Per-version release notes. |
## Relationship to the existing project

m5Tui is a **sibling** to `/home/pi/m5-cardputer-adv` (the capture-side
project already on `main` at v0.6.1).

- `m5-cardputer-adv` is the **capture** surface — record ideas, voice
  memos, tasks on the device, sync them to the bridge.
- **m5Tui** is the **control** surface — drive the agent ecosystem,
  run OMP, search the vault, execute plans.

They share the SD card layout, the `advdeck-bridge` CLI, and the
project folder structure. They do not share code. A future v2 may
unify them.

## Hard rules

1. All new code must pass `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt -- --check`.
2. Real hardware/network integration (SSH over Tailscale, I2S audio, SD-card atomic writes, GitHub Pages market backend) is operator work outside the Pi toolchain.
3. Do not modify `/home/pi/m5-cardputer-adv/` from this project.

---

**Owner:** Forest
**Drafted:** 2026-06-15
**Last sync:** 2026-06-17 (v1.11.0 real device drivers + market backend)
**Target device:** M5Stack Cardputer-Adv (K132-Adv, ESP32-S3)
**Target server:** `aiserver-1` (Pi 5, Tailscale 100.126.207.73)
**License:** MIT
