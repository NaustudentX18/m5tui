# m5Tui

> **m5Tui v1.0.0 — workspace is structurally complete.** All roadmap milestones (M0–M6 + v1.0) are implemented as trait-based stubs with unit/sim tests. Real hardware/network integration is the remaining operator work.

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

🟢 **v1.0.0 STRUCTURALLY SHIPPED — 2026-06-16** — M0 foundation, M1 cockpit shell, M2 theme engine, M3–M6 trait stubs, and v1.0 integration. Native `cargo test --workspace` is green. See `CHANGELOG.md` and `ROADMAP.md`.

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
**Last sync:** 2026-06-16 (v1.0.0 structural complete)
**Target device:** M5Stack Cardputer-Adv (K132-Adv, ESP32-S3)
**Target server:** `aiserver-1` (Pi 5, Tailscale 100.126.207.73)
**License:** MIT

