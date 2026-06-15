# m5Tui — Pocket Agent Cockpit for the Cardputer-Adv

**Phase:** Planning (pre-implementation, awaiting SWAT approval)
**Owner:** Forest
**Drafted:** 2026-06-15
**Last sync:** 2026-06-15 (post-SWAT round 1)
**Status:** 🟢 SWAT APPROVED — 2026-06-15. SWAT decisions applied as-proposed (§13.2). Proceeding to M0.

> **One-liner:** A sexy, keyboard-first, themable CLI/TUI that lives on the
> Cardputer-Adv and SSHs into `aiserver-1` (Pi 5) so you can run and steer
> the agent ecosystem exactly like you'd use Termius on your phone — except
> it understands agents, voice, the device, and ships with a Book of
> Commands and a Community Theme Market.

---

## 0. TL;DR for the SWAT

| Field | Value |
|---|---|
| Folder | `/home/pi/m5Tui/` (new, created 2026-06-15) |
| Stack (locked) | **Rust + Ratatui + Crossterm** with `russh` for SSH, `cpal`/ESP-I2S for mic, `serde_yaml` for themes. Standalone device-native binary. |
| Run target | M5Stack **Cardputer-Adv (K132-Adv)** — ESP32-S3, 240×135 ST7789V2, 56-key keyboard, ES8311 codec, MEMS mic, IR, BMI270, 8 MB flash, microSD, 1750 mAh |
| Network | **Tailscale mesh** (primary) + **direct SSH** (fallback when no mesh). Cardputer hosts its own Tailscale identity. |
| Talks to | `aiserver-1` (Pi 5) at Tailscale 100.126.207.73 / LAN 192.168.4.91; `desktop-ujsii52` PC Ollama at Tailscale 100.127.91.97:11434 / LAN 192.168.4.69:11434 |
| OMP RPC pin | OMP **v15.13.3** (current `omp update`-ed version) — auto-checked at boot |
| What it does | Standalone SSH multiplexer + agent cockpit + voice PTT + command palette + on-device theming + **Book of Commands** + **Community Theme Market** client. Keyboard-driven, 240×135-first. |
| Repo | **New repo** `https://github.com/NaustudentX18/m5tui` |
| License | **MIT** (matches `m5-cardputer-adv`) |
| Pre-installed themes | 6: `Coldwire`, `Phosphor`, `Lacuna`, `Magline`, `Noctilux`, `Ivoryroom` (names read like their look) |
| Out of scope (v1) | Host-side bridge / planner logic (already in `m5-cardputer-adv` v0.6.1); running the LLM on the device; BLE HID; LoRa; GPS; multi-user auth |
| Verifies as done when | Milestone deliverables in §10 all green and the Cardputer boots standalone into the cockpit, joins Tailscale, SSHs into aiserver-1, runs an OMP session, records/pushes mic input, switches themes live, persists state across reboots, and the Book of Commands works one-handed |

### 0.1 SWAT decisions locked in (2026-06-15)

| # | Question | Decision |
|---|---|---|
| 1 | Tech stack | **Rust + Ratatui + russh. No Go/Bubble Tea prototype.** |
| 2 | Device vs relay | **Device-native v1, standalone.** Tailscale on the device, direct SSH as offline fallback. |
| 3 | OMP RPC version | **OMP 15.13.3** pinned; auto-checked at boot. |
| 4 | Theme editor in M2 | **Yes, M2 as planned.** |
| 5 | Sound default | **On by default, themable + toggle.** |
| 6 | Repo | **New `NaustudentX18/m5tui`.** |
| 7 | License | **MIT.** Doesn't matter much for a personal project but matters for forks. |
| 8 | First theme | **Renamed** — see `THEMING.md` for the 6 names. Default is `Coldwire` (cyberpunk cyan). |
| 9 | Voice in v1 | **Yes, full PTT in M5.** |
| 10 | Sound effects | **Generated in code** (procedural square/sine WAVs); themes override with their own files. Toggle per theme. |
| **+** | New: Book of Commands | **Required.** One-key (or one-prefix) library of named commands, discoverable, with help and parameter prompts. |
| **+** | New: Community Theme Market | **Required.** Browse / preview / install community themes. Needs internet (Tailscale). |

### 0.2 New questions surfaced by these decisions

See §13.2.

---

## 1. Why this exists

You have:

- **A Pi 5 aiserver** (Tailscale `aiserver-1`, 16 GB) running OMP, `omp-memory`, multiple model presets, MCP servers, and a `claude-mem` vault.
- **A PC Ollama box** (RTX 4070 Ti, `desktop-ujsii52`) that runs the heavy models.
- **A Cardputer-Adv** in your pocket — 240×135, 56 keys, mic, speaker, IR, IMU, 1750 mAh.
- **A working capture-side project** at `/home/pi/m5-cardputer-adv` v0.6.1 (the firmware + bridge that turns rough ideas into agent handoff packs).
- **No way to drive any of this from the Cardputer today.** You have to pull out the phone, open Termius, type a command, get a wall of text, then squint. The "Termius on the phone" loop is the bottleneck.

**m5Tui replaces Termius with something that knows what an agent is.** It's a TUI that's tuned for the Cardputer's screen and keys, that speaks SSH natively, that has a built-in model of "agent profiles," that can use the on-board mic as a first-class input, and that ships with a **Book of Commands** and a **Community Theme Market** so you don't have to memorise keys and the visual identity can keep evolving.

The design ethos is:

1. **Sexy, fun, and unmistakably yours.** Cyberpunk/CRT/glitch aesthetic is the default, but every pixel is themable from the device itself and the community can publish themes.
2. **Keyboard is the only input that matters.** Mouse, trackball, even the IR emitter are bonus, not required. The Book of Commands makes every action one keystroke.
3. **Designed for one-handed thumb typing on a 56-key grid.** No chords that need a manual; the hotkey overlay is always one key away.
4. **Everything stateful lives on the SD card as plain text.** No SQLite, no binary blobs you can't grep.
5. **The agent is the abstraction, not the shell.** You don't `ssh` and then `claude` — you `cast` a command and the TUI handles the rest.
6. **Standalone.** The Cardputer is not a thin client. It hosts its own Tailscale, its own mic capture, its own theme engine, its own scrollback. It works offline (with the LAN fallback profile) and degrades gracefully.

---

## 2. Scope, audience, non-goals

### 2.1 In scope (v1)

- A single Rust binary `m5tui` that:
  - Runs **standalone on the Cardputer-Adv** (no Pi-side relay required)
  - Hosts its own **Tailscale** identity so the device is reachable on the mesh without port forwarding
  - Connects to `aiserver-1` over Tailscale mesh (primary) or direct LAN SSH (fallback when the mesh is down)
  - Manages a **profile registry** of SSH hosts (aiserver-1, desktop-ujsii52, anything else) with keys, ports, jump hosts
  - Embeds an **SSH terminal pane** for arbitrary shell commands
  - Speaks the **OMP JSON-RPC protocol** (omp `--mode rpc` / `--mode rpc-ui` in v15.13.3) so you can drive a real OMP session from the device
  - Has a **command palette** (fuzzy) bound to `;/` and to single-key shortcuts
  - Has a **Book of Commands** — a discoverable, one-key (or one-prefix) library of named commands ("Cast", "Forge", "Heal", "Banish", etc.) with help text, parameter prompts, and undo. See `COMMANDS.md`.
  - Has a **theme system** with on-device editor — pick colours, glyphs, layout density, animations. Ships with 6 pre-installed themes. See `THEMING.md`.
  - Has a **Community Theme Market** client — browse, preview, and install themes uploaded by other m5Tui users. See `MARKET.md`.
  - Has a **voice / push-to-talk** layer that captures mic, encodes WAV, and either pushes the file to the server or streams it to a local STT bridge
  - Has **persistent session state** — last connected host, last agent, scrollback buffer, theme, profile, command history
  - Has **on-screen status** — Wi-Fi signal, battery, Tailscale mesh state, SSH connection health, agent thinking/streaming state, CPU/RAM of remote
  - Has an **agent handoff** flow: select a remote project under `/mnt/hdd/knowledge-base` or `~/projects`, view the agent pack, continue the conversation
  - Has a **macro / snippet** system: type a few letters, expand to a full agent command
  - Has a **log pane** that tails the OMP session log + the bridge log + the OMP memory log

### 2.2 Out of scope (v1)

- **Running the LLM on the device.** The Cardputer has no PSRAM, 8 MB flash, and a 240 MHz Xtensa LX7 — it is a terminal, not a model host. The remote is always the brain.
- **Replacing the existing `m5-cardputer-adv` firmware.** That project is the *capture* surface; m5Tui is the *control* surface. They share a data model and an SD card layout but they are separate binaries. (A future v2 may unify them — see §10.)
- **BLE HID emulation.** The Cardputer has it but pairing is fragile. Out of scope.
- **LoRa / GPS / RFID / IR macros.** Out of scope for v1. IR is a *display* theming tool, not a transmitter. *(See §5.6 — IR as a UI element.)*
- **Multi-user / multi-tenant auth.** This is a personal device. v1 assumes one operator.

### 2.3 Audience

- **Primary:** Forest. You're the only user. Every design decision optimizes for the way *you* work.
- **Secondary:** Future you, looking at this in six months. Make it obvious.
- **Tertiary:** Any other Cardputer owner who wants a copy. The license is **MIT** from day one (matches `m5-cardputer-adv`); the repo lives at `https://github.com/NaustudentX18/m5tui`.

---

## 3. Existing assets we are *not* rebuilding

m5Tui deliberately leans on work already shipped:

| Asset | Path | What m5Tui gets from it for free |
|---|---|---|
| `m5-cardputer-adv` v0.6.1 | `/home/pi/m5-cardputer-adv` | Bridge protocol, project folder layout (`brief.md`, `plan.md`, `tasks.json`), `advdeck-bridge` CLI on the server side, the 288-test regression net |
| OMP v15.13.3 | `/home/pi/.omp/` | The `--mode rpc` and `--mode rpc-ui` modes give us a real agent control plane out of the box — we just need a TUI that speaks JSON-RPC to it |
| `omp-memory` CLI | `~/.omp/bin/omp-memory` | Vault search from the device (`m5tui vault <query>` → renders hits in the scrollback) |
| Tailscale identity on `aiserver-1` | 100.x mesh | The server is already on the mesh; the Cardputer joins the same tailnet in v1. |
| PC Ollama at `desktop-ujsii52:11434` | 12 GB VRAM | Used as a *direct model endpoint* for the voice/STT path if we don't want to round-trip through the Pi for everything |
| `claude-mem` vault | `/mnt/hdd/knowledge-base/claude-mem` | Reading past sessions from the device — implement `m5tui mems <query>` and it just works |
| `obsidian-memory` CLI | in `$PATH` | Same idea, Obsidian-flavored view of the vault |

**Do not fork or reimplement any of these.** m5Tui is a *consumer* of the aiserver ecosystem, not a replacement for it.

---

## 4. High-level architecture

```
+----------------------------------------------------------+
|  M5Stack Cardputer-Adv (ESP32-S3, 240x135 ST7789)        |
|  +-----------------------------------------------+       |
|  | m5tui (Rust binary, 1.2-1.8 MB stripped)     |       |
|  |   +-------+   +--------+   +-----------+     |       |
|  |   | TUI   |   |  SSH   |   |   Voice   |     |       |
|  |   | core  |<->| client |<->|  (mic+    |     |       |
|  |   | (rata-|   |(russh) |   |  codec)   |     |       |
|  |   | tui)  |   +--------+   +-----------+     |       |
|  |   |       |        |            |            |       |
|  |   | theme |   OMP RPC bridge    WAV->SD      |       |
|  |   | eng.  |   (JSON-RPC over    push         |       |
|  |   | Book  |    SSH channel)     scp          |       |
|  |   | of    |                                   |       |
|  |   | Cmds  |   Market client (HTTPS)           |       |
|  |   +---+---+                                   |       |
|  |       ^                                        |       |
|  |       v                                        |       |
|  |   SD: /sd/m5tui/{config,profiles,themes,       |       |
|  |       history,snippets,memos,book,market}/*    |       |
|  +-----------------------------------------------+       |
|       ^  WiFi STA                                      |
|       |  Tailscale userspace daemon (host identity)    |
+-------|--------------------------------------------------+
        |
        v
+--------------------------------------------+
| Tailscale mesh (100.x)                    |
|   - aiserver-1  (Pi 5)                    |
|   - desktop-ujsii52 (PC, Ollama)          |
|   - m5tui-XXXXXX (Cardputer, this device) |
+--------------------------------------------+
        |
        |  (or direct LAN if mesh is down)
        v
+--------------------------------------------+
| aiserver-1 (Pi 5)                         |
|   - sshd (key auth)                       |
|   - omp, omp-memory, omp-doctor, ...      |
|   - advdeck-bridge plan / export / retry  |
|   - claude-mem/ vault                     |
|   - ollama on desktop-ujsii52 (proxied)   |
+--------------------------------------------+
```

### 4.1 Module map

```
m5tui/
  crates/
    m5tui-core/        # event loop, app state, ratatui root, keymap
    m5tui-ssh/         # russh client, channel multiplexing, keepalive
    m5tui-omp/         # OMP RPC client (parses --mode rpc, sends JSON-RPC)
    m5tui-voice/       # mic capture (cpal-like shim), WAV encoder, push channel
    m5tui-themes/      # theme parser, color/blink/anim engine, live preview
    m5tui-pal/         # command palette, fuzzy matcher, snippet expander
    m5tui-book/        # Book of Commands: named commands, params, undo, help
    m5tui-market/      # Community Theme Market client (HTTPS, JSON catalog)
    m5tui-profile/     # profile registry, jump host, keychain, ssh-agent shim
    m5tui-persist/     # SD layout, atomic writes, schema migration
    m5tui-status/      # Wi-Fi, battery, Tailscale mesh, SSH health, doctor
  src/main.rs          # tiny bin, just bootstraps core
  themes/              # default theme .yaml files, ship in binary
  book/                # default command book, ships in binary
  market/              # local mirror of the market catalog (cached)
  tests/
    unit/              # parser/theme/ssh-handshake tests
    e2e/               # spawn local sshd, run a fake OMP server
    sim/               # simulate a 240x135 grid, screenshot output
```

### 4.2 Why workspace, not single crate

The Cardputer binary is a single `m5tui` ELF, but splitting the code into a Cargo workspace means:

- Each module can be unit-tested on the Pi without flashing the device
- We can swap `m5tui-voice` for a stub during headless CI
- The Market client (`m5tui-market`) is a pure HTTP module that can be unit-tested without any of the TUI machinery
- The Book of Commands (`m5tui-book`) is a pure data structure that can be fuzz-tested in isolation
- Future extraction to a desktop TUI (for testing on the Pi itself) is a one-line `[[bin]]` away

### 4.3 Cardputer-specific constraints that drive every design choice

- **240×135 px, 16-bit color.** A TUI cell is 6×8 or 8×8 px; we get ~30 cols × 17 rows at 6×8 or ~40×16 at 6×8 with a 6 px side margin. Ratatui handles this with the `set-viewport-size` trick, but our layouts are tuned for 40×16.
- **56 keys, 4×14 matrix, no Fn row.** Hotkeys live on the symbol layer. The `;` key (top-left) is our `Esc`. The `'` key is `Tab`. The `\` key is `Backspace` (it's the only backspace-shaped key). Modifier chords are expensive — single-key bindings are king. **The Book of Commands is the answer to "how do I remember all this?"** Every command has a single-key or single-prefix binding.
- **No PSRAM, 8 MB flash.** Stripped binary target is **< 1.8 MB**. No embedded regex engine, no embedded font rasterizer; ship a 6×8 ASCII font as a const array.
- **ES8311 codec, MEMS mic, NS4150B speaker.** Audio capture is possible (the existing `m5-cardputer-adv` proves it for voice memos). Streaming STT is a v1 stretch goal — v1 ships with **mic-to-WAV-to-server-push** only.
- **Wi-Fi only, no ethernet.** Tailscale userspace daemon on the device means `aiserver-1` resolves via MagicDNS. We *also* fall back to LAN IP `192.168.4.91` if Tailscale is down.
- **1750 mAh battery.** Brightness is a themable; default to 60% in `Coldwire`, 30% in `Lacuna`. Auto-dim on idle (configurable).
- **No real-time clock.** Sessions are timestamped on the server. The device shows "since last sync" instead of a wall clock until NTP sync lands.

---

## 5. Feature inventory

### 5.1 The cockpit (the home screen)

When m5Tui boots, you land on the **cockpit** — a single-screen dashboard.

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
| PALETTE  ;/  |  VOICE  ;v  |  THEME  ;t     |
+--------------------------------------------+
| > _                                                |
+--------------------------------------------+
```

- Top bar: server status, Tailscale mesh state, uptime, battery, signal, last sync.
- Left pane: **agent registry** — every SSH host, every OMP profile, every project workspace. Arrow keys + enter to focus.
- Right pane: **live OMP session state** — current model, token rate, context window fill, in-flight task queue, last error.
- Bottom: prompt line. Typing there enters *agent mode* (sends to OMP).
- `;/` opens the command palette.
- `;v` enters voice push-to-talk.
- `;t` opens the theme editor.
- `;b` opens the Book of Commands.
- `;m` opens the Community Theme Market.
- `;?` opens the help / hotkey overlay.

### 5.2 SSH + agent control

- **One-key connect.** `;c` opens a small "connect to…" picker; `enter` connects; `;d` disconnects; `;r` reconnects with keepalive.
- **Tailscale identity.** On first boot, the wizard asks for a Tailscale auth key and registers the device as `m5tui-<short-id>`. The device then appears in the mesh as a peer; the aiserver can SSH *to* it if needed (handy for remote diagnostics).
- **Direct-SSH fallback.** If the mesh is down (`tailscaled` not running, no internet), m5Tui falls back to the LAN address (`192.168.4.91`). The top bar shows `mesh: down :: lan-fallback`.
- **Inline terminal pane.** Anything you type that isn't a `;`-prefixed hotkey goes to the remote PTY. Output is rendered through a `less`-style pager with `j/k/g/G` and `/` search.
- **OMP-aware mode.** When the remote session is an OMP `--mode rpc-ui`, the TUI upgrades itself from "dumb terminal" to "agent cockpit" — it parses the JSON-RPC frames and renders them as structured widgets (tool call cards, file diffs, todo lists, subagent tree).
- **Agent profiles.** A profile = (host, ssh key, jump host, working dir, default model, system prompt prefix, snippet library). `;a` opens the profile picker. Profiles live in `/m5tui/profiles/*.yaml` on SD.
- **Handoff.** `;h` opens the handoff viewer — reads `agent-prompt.md` from the focused project's folder on the server, renders it as Markdown, lets you "continue" the conversation in a new OMP session.
- **Vault search.** `;m` opens the OMP memory / claude-mem search box. `m5tui mems "cardputer mic"` works from the palette; results are streamed in and rendered as a hit list with `enter` to expand.

### 5.3 Voice / push-to-talk

- **Hold `;v` to record.** Mic captures at 16 kHz, 16-bit mono, encoded to WAV in-memory. On release, the WAV is written to `/m5tui/voice/<timestamp>.wav` on the SD card.
- **Auto-push.** The TUI immediately `scp`s the file to `~/voice-inbox/` on the aiserver and (optionally) enqueues an `advdeck-bridge plan` job with `transcript.md` as the input.
- **Live transcript overlay.** If a streaming STT endpoint is available (e.g. an `omp serve` sidecar running `whisper.cpp` on the PC), a one-line transcript preview appears under the prompt while you talk. **v1 stretch goal** — ship without, add in M5a.
- **Playback.** `;p` plays the last captured voice memo through the ES8311 → NS4150B speaker. Volume is a themable slider.

### 5.4 Book of Commands  [see COMMANDS.md]

A first-class, on-device, discoverable library of named commands. Each command has:

- A **name** (e.g. `cast`, `forge`, `heal`, `banish`)
- A **single key** or **single prefix** binding (e.g. `;c` for `cast`, `;f` for `forge`)
- A **category** (agent / shell / voice / theme / market / handoff / system)
- A **description** (one line, shown in the palette)
- An **expanded command** — the actual command(s) it runs
- **Parameter prompts** (optional) — for commands that need input
- **Help text** (optional) — a longer explanation
- **Undo** (optional) — a command to run on `;u` to reverse the last action

The book ships with ~30 commands. The user can add more on the device. The book is data (`/m5tui/book/<name>.yaml`), not code — the same "data, not code" principle as the theming system.

Full spec: `COMMANDS.md`.

### 5.5 Theming + on-device personalisation  [see THEMING.md]

m5Tui ships with **6 pre-installed themes** whose names *read like the look*:

| Name | Vibe | What you see when you read the name |
|---|---|---|
| `Coldwire` | Cyberpunk cyan on midnight | cold + wire = a thin blue line through a frozen data stream |
| `Phosphor` | Amber CRT, vt220 vibes | phosphor = the glow on the back of an old CRT |
| `Lacuna` | Dim red-on-black, almost off | lacuna = a gap, a missing piece, a hole in the light |
| `Magline` | Magenta on indigo, synthwave grid | magenta + line = a horizon line of pink light |
| `Noctilux` | High-contrast, legibility-first | noctilux = "night light" — the brightest thing in the dark |
| `Ivoryroom` | White on cream, paper-notebook feel | an ivory room = quiet, blank, nothing hidden |

Full spec: `THEMING.md`.

### 5.6 Community Theme Market  [see MARKET.md]

A network-accessible catalog of community-uploaded themes.

- `;m` opens the market.
- Browse: list view with name, author, palette swatch, glyph preview, popularity, install count.
- Preview: download the YAML locally and render a live preview pane in the cockpit using the *current* project's data.
- Install: write to `/m5tui/themes/<name>.yaml` and the picker picks it up immediately.
- Publish: `;m p` opens "publish my current theme" — uploads to the market with a one-line description and your handle.
- The market is an HTTPS GET/POST against a small static-JSON + object-storage backend (GitHub repo for the catalog, S3-compatible for theme YAMLs and WAVs).
- Offline: the catalog is cached at `/m5tui/market/catalog.json`; the picker still works without internet.

Full spec: `MARKET.md`.

### 5.7 Persistence

Everything m5Tui stores lives under one root on the SD card:

```
/m5tui/
  config.yaml           # global settings, default profile, theme, brightness
  profiles/             # one YAML per SSH host / agent profile
    aiserver-1.yaml
    desktop-ujsii52.yaml
    pc-ollama-direct.yaml
  themes/               # user themes
    coldwire.yaml
    phosphor.yaml
    lacuna.yaml
    magline.yaml
    noctilux.yaml
    ivoryroom.yaml
    mine.yaml
  book/                 # Book of Commands
    cast.yaml
    forge.yaml
    heal.yaml
    banish.yaml
    ...
  market/               # local market cache
    catalog.json
    installed.json
  history/              # per-host scrollback + command history
    aiserver-1/
      scrollback.arrow  # ring buffer, capped at 256 KB
      commands.jsonl
  snippets/             # per-profile text expanders
    aiserver-1/
      plan.yaml         # `;plan` -> "advdeck-bridge plan --project {p}"
      retry.yaml
  voice/                # captured WAVs before push
    2026-06-15T22-04-11Z.wav
  memos/                # local-only quick notes, never pushed
    2026-06-15.md
  logs/
    m5tui.log           # JSON-lines, auto-rotated at 512 KB
  cache/                # ephemeral; wiped on boot
    market/
    dns/
```

- All writes are **atomic** (write-temp + rename).
- All paths are configurable; the default root is `/sd/m5tui/` but you can move it to `/advdeck/m5tui/` to share the SD layout with the capture-side app.
- Schema migrations: every YAML carries a `schema_version: 1` key; bump and add a migration in `m5tui-persist`.

### 5.8 Hardware status

- **Battery %.** AXP2101 power monitor (the Cardputer-Adv's PMIC). m5Tui polls it every 30 s; the percentage is rendered in the top bar.
- **Wi-Fi RSSI.** Polled from the ESP-IDF Wi-Fi API; rendered as `▁▃▆█` bars.
- **Tailscale mesh state.** Polled from `tailscale status` (via `m5tui-status`); rendered as `mesh: ok` / `mesh: joining` / `mesh: down`.
- **Charging state.** Shown as a `⚡` next to the battery % when charging.
- **IMU tap-to-wake.** A double-tap on the Cardputer (BMI270 interrupt) wakes the screen from off. Configurable in the theme editor.
- **IR as a UI element.** Hold `;i` to "flash" the current theme's accent colour through the IR emitter — handy for testing IR receivers in the room. Pure novelty, but it fits the "sexy and fun" brief.

### 5.9 Help system / discoverability

- `;?` opens the **hotkey overlay** — a full-screen modal listing every binding, grouped by pane.
- The **first-boot wizard** walks you through: Wi-Fi creds, Tailscale auth key + registration, SSH key generation, aiserver host, default theme, mic test.
- **Tooltips.** Hovering on the agent name in the top bar shows the full hostname + last error in a one-line callout.
- **`m5tui doctor`** is a self-check that runs on boot: Wi-Fi OK? Tailscale mesh OK? SSH handshake OK? aiserver disk space? OMP responds? Returns a green/yellow/red scoreboard you can screenshot.

---

## 6. Tech stack — locked

**Decision:** Rust + Ratatui + Crossterm + russh. No prototype phase. Standalone device binary.

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| **Rust + Ratatui + Crossterm + russh** | Single static-ish binary ~1.5 MB; ratatui is the best TUI library in any language; russh is pure-Rust SSH; serde_yaml for themes; tokio for async. | ESP32-S3 cross-compile is fiddly (uses `esp-rs`/`std` Xtensa toolchain, not just `cargo build`); no PSRAM means no embedded regex; ratatui is text-cell not pixel — we lose sub-cell precision. | **Locked.** |
| Go + Bubble Tea + x/crypto/ssh | Easier cross-compile. | 5-10 MB binary, eats battery, ESP32 port not realistic. | **Rejected.** |
| MicroPython on the device, Rust on a Pi relay | Quick to prototype. | Dynamic typing kills a TUI this complex; performance ceiling; we explicitly want standalone. | **Rejected.** |
| C++ + FTXUI or LVGL | LVGL gives pixel-perfect control on the ST7789V2. | LVGL is a *GUI* not a TUI — we'd reinvent panes, focus, modals. C++ toolchain is heavier than Rust. | **Rejected** unless we discover a layout that the 40×16 grid can't express. |
| Pure web (SoftAP + browser) | Easiest UI. | Loses keyboard-first, form factor, battery life, offline-ness, the "sexy" brief. | **Rejected** for v1. Park for v2 as a *companion* surface. |

**Sub-decisions baked in:**

- **`ratatui` over `crosstui`:** the active fork with the bigger ecosystem.
- **`russh` over `ssh2`:** pure-Rust, no libssh C dependency, easier on the cross-compile.
- **`cpal` replaced by an ESP-I2S shim:** the device-side audio path is hand-written, not cpal. Desktop testing uses cpal; on-device uses a tiny `esp-hal` I2S driver wrapped in the same trait.
- **`serde_yaml` for themes, `toml` for config:** themes are user-edited and YAML is friendlier; config is machine-tweaked and TOML is stricter.
- **`tokio` for async:** ratatui uses it; russh uses it; the only argument is embassy on bare metal, but we don't need that — we run on top of the Cardputer's existing firmware-style runtime (or a dedicated ESP-IDF task that hosts our `std` binary via `esp-idf-sys`).
- **`ureq` (sync) over `reqwest` (async) for the Market client:** Market calls are infrequent and the response fits in a single request. Sync = simpler code = smaller binary.

---

## 7. UX details — the "sexy and fun" brief, made concrete

The visual identity is **CRT-cyberpunk by default** (`Coldwire` is the hero theme), with a heavy lean on:

- **Glyph work.** Heavy box drawing, half-block sparklines for RSSI/battery/CPU, braille-pattern progress bars (looks great at 6×8). The 8-bit glyph atlas ships in the binary.
- **Scanline overlay.** Optional `1px` alternating darken on every other line. Toggle in theme.
- **Phosphor decay.** Text fades from accent → dim → fg over 400 ms when a new event lands. Optional. Expensive on the ESP32-S3 but doable at 30 Hz.
- **Glitch on agent events.** When the OMP RPC stream reports a `tool_call`, the title bar jitters for 200 ms. Tiny visual reward for "the agent is working."
- **Animated prompt.** Idle prompt is `_`. While the agent is streaming, it cycles through `▁▂▃▄▅▆▇█` (a real "loading bar"). The cycle speed is tied to the token rate.
- **Synthwave horizon.** The status bar at the bottom can be configured to render a perspective grid of lines receding to a vanishing point. Pure decoration. Looks incredible.
- **Sound design.** Every theme has a 3-note "boot" chime, a click on every key, a low thunk on disconnect, an ascending arpeggio on successful agent handoff. The chime and arpeggio are procedurally generated by default; themes can override with their own WAVs. Per-theme toggle.
- **Ascii art boot screen.** The boot screen is a 40×16 ASCII rendering of the m5Tui logo with a fake "loading" animation. Skippable with `enter`.

All of the above are **themable**. The `Coldwire` theme is heavy on synthwave; `Phosphor` is amber-on-black with no scanlines; `Lacuna` is pure red with no animation. New themes can be authored on the device, exported as one YAML file, and **published to the Community Theme Market** with one key (`;m p`).

---

## 8. Data model

```yaml
# config.yaml
schema_version: 1
default_profile: aiserver-1
theme: coldwire
brightness: 60                # 0-100, drives G38 PWM
animations: 1                 # 0..2
sound: true
storage_root: /sd/m5tui
tailscale:
  enabled: true
  hostname: m5tui-cardputer
  fallback_lan: 192.168.4.91
omp:
  binary: omp
  pinned_version: 15.13.3
  extra_env:
    PI_SMOL_MODEL: phi4-mini
  default_args: ["--mode", "rpc"]
voice:
  push_target: ~/voice-inbox/
  auto_enqueue_plan: true
  stream_stt_url: ""          # empty = no live transcript in v1
market:
  catalog_url: https://m5tui.community.market/catalog.json
  enable_telemetry: false     # opt-in
book:
  custom_dir: /sd/m5tui/book
```

```yaml
# profiles/aiserver-1.yaml
schema_version: 1
host: aiserver-1
address: aiserver-1           # MagicDNS via Tailscale
address_fallback: 192.168.4.91
port: 22
user: pi
identity_file: /sd/m5tui/keys/aiserver-1_ed25519
jump_host: null
working_dir: /home/pi
omp:
  enabled: true
  binary: omp
  startup_cmd: omp --mode rpc
  pinned_version: 15.13.3
model_default: minimax
snippets_dir: /sd/m5tui/snippets/aiserver-1
```

```yaml
# themes/coldwire.yaml
schema_version: 1
name: coldwire
author: forest
palette:
  bg:        "#0a1428"
  fg:        "#d0e0ff"
  accent:    "#00d8ff"
  dim:       "#506a8a"
  warn:      "#ffaa00"
  err:       "#ff3050"
  ok:        "#44ee88"
  sel_bg:    "#1a3868"
  sel_fg:    "#ffffff"
  prompt:    "#00d8ff"
glyphs:
  box: heavy
  scrollbar: block
  cursor: block
layout:
  density: compact
  show_clock: false
  show_synthwave: true
animation:
  level: 1
  scanline: true
  phosphor_decay: false
  glitch_on_event: true
sound:
  enabled: true
  boot:  /sd/m5tui/themes/coldwire/boot.wav
  click: /sd/m5tui/themes/coldwire/click.wav
  arp:   /sd/m5tui/themes/coldwire/arp.wav
brightness: 60
```

```yaml
# book/cast.yaml
schema_version: 1
name: cast
category: agent
description: "Ask the OMP agent a one-shot question"
binding: ";c"
params:
  - name: prompt
    message: "ask?"
    multiline: true
template: |
  ;ask {prompt}
help: |
  Cast opens the OMP prompt pre-filled with your question. Long-press
  ;c to open with a multi-line input for longer briefs.
undo: null
```

Schema is enforced at load with `jsonschema` (compiled-in, no runtime fetch). Invalid YAMLs are rejected with a friendly error and the previous good config is kept.

---

## 9. Build, test, distribution

### 9.1 Build

- **Cargo workspace** as laid out in §4.1.
- **Device build:** `cargo build --target xtensa-esp32s3-espidf --release` (via `esp-rs`).
- **Host build (for testing on the Pi):** `cargo build --release` → `m5tui` binary, runs in a normal Linux PTY.
- **CI:** GitHub Actions matrix builds (xtensa + native), runs the unit + e2e + sim tests.

### 9.2 Test strategy

- **Unit tests** for every parser, every theme load, every book command expansion, every snippet expansion, every RPC frame decode.
- **E2E tests** spawn a local `sshd` in a container, run a fake OMP server that speaks `--mode rpc`, run a fake Market HTTPS server, and drive the TUI with synthetic keystrokes via `crossterm::event::Event::Key` injection. Screenshots are diffed against golden PNGs.
- **Sim tests** render the TUI to a 240×135 in-memory framebuffer and assert the output. This is how we lock down "the boot screen looks like this" and "the help overlay is 17 rows, no overflow."
- **Live smoke** on a real Cardputer-Adv, run by hand: `cargo run --release` flashed via `mpremote` or `esptool.py`, walk the first-boot wizard, join the Tailscale mesh, connect to aiserver-1, run an OMP command, capture a voice memo, switch themes, browse the Market.

### 9.3 Distribution

- **GitHub repo:** `https://github.com/NaustudentX18/m5tui` (new repo).
- **Releases:** prebuilt `m5tui-esp32s3.bin` and `m5tui-linux-arm64` attached to GitHub releases. Tag scheme: `v0.1.0`, `v0.2.0`, …
- **First-boot wizard** baked into the binary so a new device is one flash + one screen of setup away from useful.
- **Theme + Book gallery:** the repo ships with the 6 pre-installed themes + ~30 default Book commands in `/themes/` and `/book/`. Users can `git pull` to get more.
- **Market backend:** a small static site on GitHub Pages (catalog as JSON) + a free-tier object store (R2 / S3 / Backblaze B2) for theme YAMLs and WAVs. See `MARKET.md` §3.

---

## 10. Milestones — what gets built in what order

| Milestone | Name | Ships when | Verification |
|---|---|---|---|
| **M0** | Foundation | Repo scaffolded; Cargo workspace builds for host + xtensa; empty `m5tui` binary boots on the Pi and renders a 40×16 "hello" string | `cargo run` on Pi prints "m5Tui v0.1.0" in a 40×16 framebuffer. xtensa build produces a `.bin` that flashes. |
| **M1** | Cockpit shell | Home screen renders with mock data: top bar, agent list (mock profiles), session state (mock), prompt line. `;/` opens a static command palette. `;?` opens a static help overlay. | Unit test: render in sim, screenshot matches golden. E2E: keystrokes cycle the focus. |
| **M2** | Theme engine + editor | **6 built-in themes** ship (`Coldwire`, `Phosphor`, `Lacuna`, `Magline`, `Noctilux`, `Ivoryroom`); theme editor lets the user change palette / glyphs / density / brightness live; live preview updates as you change; save reloads after reboot. | Unit: every theme YAML parses; invalid YAML is rejected with a friendly error. E2E: change `accent` in editor, restart, accent is still changed. |
| **M3** | SSH + Tailscale + profiles | Cardputer joins Tailscale mesh; connect to aiserver-1 over Tailscale; direct-SSH LAN fallback works; render the remote PTY; scrollback captures and persists; profile registry works; first-boot wizard collects Wi-Fi/Tailscale/key. | E2E: connect to a local sshd, run `ls`, see the listing. Manual smoke: real Cardputer joins real mesh, connects to real aiserver-1, see the prompt. |
| **M4** | OMP integration | Speak JSON-RPC to `omp v15.13.3 --mode rpc`; render tool calls as cards; show model/context/rate in the right pane; support `;a` profile switch. | E2E: stub OMP server returns canned frames, verify card layout. Manual smoke: real OMP `;ask "what is 2+2"` returns a real answer. |
| **M5** | Voice / PTT | Mic captures on `;v` hold; WAV saves to SD; `scp` pushes to aiserver-1; optional `omp` job enqueued. `;p` plays back. | E2E: synthetic mic input → WAV has correct header and sample count. Manual smoke: speak into Cardputer mic, file lands in `~/voice-inbox/` on aiserver. |
| **M5a** | Book of Commands | `;b` opens the book; ~30 default commands ship; parameter prompts work; help text displays; undo works; per-profile book override. | E2E: drive the book with keystrokes, verify expanded commands match. Sim: golden screenshot of the book. |
| **M5b** | Community Theme Market | `;m` opens the market; HTTPS GET catalog; preview pane; install; publish; offline cache. | E2E: stand up a fake Market server, install a fake theme, verify it appears in the picker. |
| **M6** | Handoff + vault | `;h` opens handoff viewer (reads `agent-prompt.md` from server). `;m` opens claude-mem search. | Manual smoke: open a known handoff, see it rendered. |
| **M7** | Status + doctor | Wi-Fi, battery, charging, signal, Tailscale mesh in top bar. `;D` runs `m5tui doctor` and shows the scoreboard. | Unit: status parsers. Manual: full-screen test on a real device. |
| **M8** | Sexy polish | Boot screen, scanline overlay, synthwave horizon, glitch on event, sound design (procedural defaults + theme overrides), animation levels. | Sim: golden screenshots for all 6 themes at animation levels 0/1/2. |
| **M9** | Public beta | README, install instructions, theme + book gallery, MIT license, GitHub release `v0.9.0`, Market backend live. | Repo public, release downloadable, install instructions work on a fresh Cardputer, Market URL serves a catalog. |
| **v1.0** | v1.0 | All M0–M9 green, no `// TODO` left in critical paths, 95% test coverage on `m5tui-persist` and `m5tui-ssh`. | All CI green. Real-device smoke checklist signed off. |

**Critical path:** M0 → M1 → M2 → M3 → M4. Everything else is parallel or polish.

---

## 11. Relationship to the existing `m5-cardputer-adv` project

m5Tui **does not replace** m5-cardputer-adv. They are siblings:

- **m5-cardputer-adv** is the *capture* surface. You use it to record ideas, voice memos, and tasks on the device, then sync them to the bridge.
- **m5Tui** is the *control* surface. You use it to drive the agent ecosystem, run OMP, search the vault, execute plans.

They share:

- The same SD card layout conventions (`/advdeck/`, `/m5tui/`).
- The same `advdeck-bridge` CLI on the server.
- The same project folder structure (`idea.md`, `brief.md`, `plan.md`, `tasks.json`).

They do not share code. A future **v2** could unify them into one binary with two modes (capture / control), but that's a v2 problem.

---

## 12. Risks and how we mitigate them

| Risk | Severity | Mitigation |
|---|---|---|
| Rust → ESP32-S3 cross-compile is fragile and slow | High | Pin `esp-idf-sys` version; pre-build a Docker image with the toolchain; CI matrix runs every PR. Get M0's xtensa build green in the first week and never touch the toolchain again. |
| Ratatui on a 240×135 screen feels cramped | Medium | Designed-for-40×16 layouts from day one; density toggle in themes; if it really doesn't work, fall back to LVGL pixel mode. |
| Mic capture on ES8311 is finicky in Rust | Medium | The existing `m5-cardputer-adv` v0.4 firmware already records WAVs in C++; mirror that driver. Stretch the abstraction into a `AudioIn` trait so we can stub. |
| SSH handshake over Tailscale adds 300-800 ms latency on first connect | Low | Persistent keepalive, channel multiplexing, store the russh session, reconnect on jitter. |
| Tailscale on the device eats battery / breaks | Medium | Tailscale is opt-in (config flag); direct-SSH LAN fallback is automatic; the wizard tests mesh status before declaring success. |
| Themes are a footgun (user creates broken YAML) | Low | Strict schema, friendly error overlay, "reset to default" button, "export theme" to share via the Market. |
| m5Tui drifts from OMP's RPC protocol if OMP bumps versions | Medium | Pin OMP version in the dev shell; `omp-compat` self-check fails the doctor if there's a mismatch. The Market also lists the OMP versions a theme has been tested with. |
| The "sexy" features eat battery | Medium | All animations and sound are themable; default theme (`Coldwire`) is animation-level 1 not 2; `Lacuna` ships with animations off for low-power use. |
| Scope creep into "Termius but better" | High | The non-goals in §2.2 are explicit. The Book of Commands and Market are bounded — see `COMMANDS.md` and `MARKET.md`. Any new request goes through the SWAT. |
| Market catalog is a single point of failure | Low | Catalog is cached locally; picker works offline; "publish" queues uploads when the mesh returns. |
| Themes from the Market could be malicious (YAML bombs, etc.) | Medium | Schema validation + size limits + a sandbox loader that refuses to execute any embedded code. Themes are pure data. |
| Book commands could escalate privilege | Low | Book runs as the same user as m5Tui (no `sudo`); commands are visible text before execution; the user confirms in the prompt. |

---

## 13. Open questions for the SWAT

### 13.1 Already decided (2026-06-15)

See §0.1.

### 13.2 Remaining questions

These are the things I'd like answers on **before** I start writing code:

1. **Default hero theme** — `Coldwire` (cyberpunk cyan) is the proposed default. OK, or do you want `Phosphor` (amber) to lead and `Coldwire` as the alternate? (Affects the boot screen, the README hero shot, the default bindings.)
2. **Book of Commands naming** — proposed command names: `cast` (ask OMP), `forge` (run advdeck-bridge plan), `heal` (retry a failed job), `banish` (kill a stuck job), `summon` (vault search), `chart` (handoff viewer), `scry` (look at logs), `weave` (edit a theme), `anoint` (apply a theme), `market` (browse themes), `voice` (PTT), `play` (playback), `wizard` (first-boot), `doctor`, `about`. Sound good, or do you want a different set?
3. **Market backend** — proposed: GitHub Pages (catalog) + Backblaze B2 / Cloudflare R2 (theme assets, free tier). OK, or do you want a different host? (You already have `aiserver-1`; we could host on it. But that creates a single point of failure for the public market.)
4. **OMP RPC pinning** — pinned to **15.13.3** for v1. Bumping requires a doctor check to pass. OK?
5. **Theme file size cap** — proposed: 16 KB per theme YAML (no embedded audio); 4 KB per WAV. Bigger themes feel bloaty on the device. OK?
6. **First-boot wizard** — 6 screens: Wi-Fi → Tailscale auth key → SSH key gen → aiserver host → default theme → mic test. OK, or do you want a different order / different steps?
7. **Theme publish gate** — should publishing a theme to the Market require GitHub auth (so handles are real), or anonymous with a handle string (so the friction is lower)? Anonymous is more inviting; GitHub auth is more honest.
8. **Sound on the default** — sound **on by default** in `Coldwire`, off in `Lacuna`, on in `Magline`, on in `Phosphor`, off in `Noctilux`, on in `Ivoryroom` (papers rustle, not clicks). OK?
9. **Book + Market telemetry** — should the binary phone home anonymous install counts ("I'm running v1.0.0 with 4 themes installed, 12 book commands") so the Market can show popularity? Or fully opt-in / off by default?
10. **Brand name for the device identity** — proposed: `m5tui-<6-char-hash>` (e.g. `m5tui-cardputer-3f8a91`). OK, or do you want a different scheme?

### 13.3 What I am *not* asking permission for

- I will **not** modify the existing `m5-cardputer-adv` project.
- I will **not** create files outside `/home/pi/m5Tui/` until the SWAT approves.
- I will **not** push to GitHub or create a new repo until the SWAT approves.
- I will **not** flash the Cardputer or touch any hardware.
- I will **not** start the Cargo workspace or write any code in this folder until the SWAT approves.

The folder exists, the planning documents live here, and that's it. Ready when you are.

---

**End of PLANNING.md. See `ARCHITECTURE.md`, `COMMANDS.md`, `MARKET.md`, `THEMING.md`, `HARDWARE.md`, `AGENTS.md`, `ROADMAP.md` for the detail.**
