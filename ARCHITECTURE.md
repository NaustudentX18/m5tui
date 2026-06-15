# m5Tui — Architecture

> Companion to `PLANNING.md`. This is the deep-dive: every crate, every
> transport, every data flow. If `PLANNING.md` is the SWAT brief, this is
> the engineering spec.

---

## 1. Runtime topology

```
+---------------------- ESP32-S3 (Cardputer-Adv) ------------------------+
|                                                                        |
|  +---------------- m5tui (single Rust process) -----------------+      |
|  |                                                              |      |
|  |  +-------+    +-------+    +-------+    +-------+    +-----+ |      |
|  |  | TUI   |    | SSH   |    | OMP   |    | Voice |    | Them| |      |
|  |  | core  |<-->| client|<-->| RPC   |<-->| (PTT) |    | eng | |      |
|  |  | (rt)  |    | (russh|    | codec |    | I2S+  |    |     | |      |
|  |  |       |    |  )    |    |       |    | WAV   |    |     | |      |
|  |  +---+---+    +---+---+    +---+---+    +---+---+    +--+--+ |      |
|  |      ^            ^            ^            ^           ^    |      |
|  |      | events     | frames     | JSON       | WAV       |YAML|      |
|  |      v            v            v            v           v    |      |
|  |  +-------+    +-------+    +-------+    +-------+    +-----+ |      |
|  |  | input |    | net   |    | net   |    | I2S   |    | persist|
|  |  | (kb,  |    | (TCP  |    | (TCP  |    | DMA   |    | (SD)  |      |
|  |  | imu)  |    |  22)  |    |  22)  |    |       |    |       |      |
|  |  +-------+    +-------+    +-------+    +-------+    +-----+ |      |
|  |                                                              |      |
|  +--------------------------------------------------------------+      |
|                                                                        |
|  Hardware: ST7789V2 (SPI) | TCA8418 (I2C kb) | ES8311 (I2S) | BMI270 |
|  Power: AXP2101 PMIC (batt %) | G38 backlight PWM                      |
|  Wi-Fi: ESP32-S3 radio (STA) | Tailscale userspace daemon              |
|  Storage: microSD SPI (CS G12)                                         |
+------------------------------------------------------------------------+
                                |
                                |  Wi-Fi (STA, Tailscale mesh)
                                v
+-------------------- aiserver-1 (Pi 5) -------------------------------+
|                                                                     |
|  sshd (port 22, key auth)                                           |
|  ├── /home/pi/omp (omp v15.13.3 binary)                             |
|  ├── /home/pi/advdeck-bridge/                                       |
|  │     └── advdeck-bridge plan / export / retry / push              |
|  ├── /mnt/hdd/knowledge-base/claude-mem/  (vault)                   |
|  ├── /home/pi/voice-inbox/  (m5Tui scp target)                      |
|  └── /home/pi/.omp/  (state, memory, skills, MCP)                   |
|                                                                     |
+---------------------------------------------------------------------+
                                |
                                |  Tailscale subnet
                                v
+-------------------- desktop-ujsii52 (PC) ----------------------------+
|  Ollama :11434  (qwen3-8b, qwen3-14b, MiniMax-M3, deepseek-r1, ...)  |
|  Used by aiserver-1 as its primary model backend.                   |
+---------------------------------------------------------------------+
```

---

## 2. Crate map

```
m5tui/                                (Cargo workspace root)
  Cargo.toml                         (workspace = [m5tui, ...])
  src/main.rs                        (tiny shim: parse args, call core::run)
  crates/
    m5tui-core/                      (UI shell, event loop, layout, widgets)
      src/
        app.rs                       (AppState, reducer)
        event.rs                     (Event enum: Key, Resize, Tick, Ssh, Omp, Audio)
        layout.rs                    (40x16 grid math, split helpers)
        widget/
          cockpit.rs
          palette.rs
          help.rs
          theme_editor.rs
          voice_overlay.rs
          toast.rs
        input/
          keymap.rs                  (semantic actions, not raw keys)
          cardputer_kb.rs            (matrix → event, 56-key layout)
        render/
          framebuffer.rs             (sim render: 240x135 RGBA, no real fb)
          ratatui_bridge.rs          (only on host build)
    m5tui-ssh/
      src/
        client.rs                    (russh::client::Handle wrapper)
        pty.rs                       (alloc-pty, resize-pty, read loop)
        keepalive.rs                 (ping every 15s, fail after 3 misses)
        known_hosts.rs               (SD-backed known_hosts file)
        scp.rs                       (push WAV to ~/voice-inbox/)
    m5tui-omp/
      src/
        rpc.rs                       (JSON-RPC over russh exec channel)
        frames.rs                    (parse tool_call, todo_update, etc.)
        render.rs                    (frame → UI events for core)
        compat.rs                    (version check, OMP version pinning)
    m5tui-voice/
      src/
        capture.rs                   (AudioIn trait, cpal impl, esp-i2s impl)
        wav.rs                       (16-bit mono WAV writer)
        playback.rs                  (AudioOut trait, cpal impl, esp-i2s impl)
        ptt.rs                       (push-to-talk state machine)
    m5tui-themes/
      src/
        schema.rs                    (jsonschema-rs compiled-in)
        loader.rs                    (parse + validate + load defaults)
        palette.rs                   (RGB565 conversion for ST7789)
        glyphs.rs                    (6x8 font, box-drawing sets, half-blocks)
        effects.rs                   (scanline, phosphor, glitch, synthwave)
    m5tui-pal/                       # command palette
      src/
        commands.rs                  # Command enum
        fuzzy.rs                     # tiny fuzzy matcher, no regex
        snippets.rs                  # text expander, profile-scoped
    m5tui-book/                      # Book of Commands
      src/
        registry.rs                  # load book/, validate, hot reload
        param.rs                     # prompt rendering, multiline, secret
        expand.rs                    # template expansion, placeholders
        undo.rs                      # undo stack + 30s timer
        author.rs                    # on-device spell editor
    m5tui-market/                    # Community Theme Market client
      src/
        catalog.rs                   # parse catalog.json
        preview.rs                   # live preview with current data
        install.rs                   # atomic install of theme + WAVs
        publish.rs                   # POST catalog entry + PUT assets
        cache.rs                     # offline catalog cache
    m5tui-profile/
      src/
        registry.rs                  # load profiles/, validate, hot reload
        connect.rs                   # resolve host, fallback LAN, Tailscale mesh
        keys.rs                      # SD-backed ssh key files, agent shim
    m5tui-persist/
      src/
        layout.rs                    # SD paths, create-on-boot
        atomic.rs                    # write-temp + rename, fsync
        migrate.rs                   # schema_version migration runner
        jsonl.rs                     # append-only logs, rotate at 512 KB
    m5tui-status/
      src/
        battery.rs                   # AXP2101 I2C poll
        wifi.rs                      # esp-wifi RSSI
        tailscale.rs                 # tailscale status, mesh state
        server.rs                    # ssh health, omp ping, disk space
        doctor.rs                    # full self-check, scored report
  themes/                            # default .yaml files, embedded
  book/                              # default spell .yaml files, embedded
  market/                            # local market cache (catalog.json)
  assets/                            # default boot/click/arp WAVs, embedded
  tests/
    unit/                            # one test file per crate
    e2e/                             # spawn sshd, fake OMP server, fake Market
    sim/                             # render to 240x135, golden diff
  docs/                              # the .md files
```

---

## 3. Event flow

The single most important design decision: **everything is an event on a single channel.**

```
input loop                 network loop              OMP RPC loop           voice loop
   |                          |                          |                     |
   v                          v                          v                     v
+-------+   +----------+   +-------+   +----------+   +-------+   +--------+
| Key   |-->|          |   | SSH   |-->|          |   | OMP   |-->|        |
| IMU   |   |  CHANNEL |   | bytes |   |  CHANNEL |   | frame |   | CHANNEL|
| Resize|   |          |   |       |   |          |   |       |   |        |
+-------+   |  (tokio  |   +-------+   |  (tokio  |   +-------+   | (tokio |
            |   mpsc)  |               |   mpsc)  |               |  mpsc) |
            |          |   +-------+   |          |   +-------+   |        |
            |          |   | SCP   |-->|          |   | STT   |-->|        |
            |          |   | done  |   |          |   | text  |   |        |
            |          |   +-------+   |          |   +-------+   |        |
            +----+-----+               +----+-----+               +----+---+
                 ^                        ^                         ^
                 |                        |                         |
                 +------ AppState reducer <+-------------------------+
                                |
                                v
                         ratatui render
```

The reducer is a pure function: `AppState -> Event -> AppState`. Every side-effecting action is implemented as "emit an Outgoing action," which the network loop picks up and dispatches. This makes the whole TUI testable in sim.

---

## 4. SSH channel layout

m5Tui opens **one SSH connection per active profile** and multiplexes channels over it:

| Channel | Purpose |
|---|---|
| `pty` (1) | The interactive shell. Used for arbitrary commands. |
| `exec` (2) | OMP RPC. `omp --mode rpc` runs in this channel; we speak JSON-RPC over its stdin/stdout. |
| `exec` (3) | `advdeck-bridge` commands. Short-lived, one per command. |
| `sftp` (4) | For `;h` (handoff viewer), `;m` (memory search result pull), and reading `agent-prompt.md` from the server. |
| `exec` (5) | `scp` for voice push. |

Channels are opened lazily and reused. If the connection drops, the keepalive task reconnects with exponential backoff (1s, 2s, 4s, 8s, 16s, max 30s) and the UI shows a "reconnecting…" toast.

### 4.1 OMP RPC protocol sketch

OMP `15.13.x` exposes a JSON-RPC surface in `--mode rpc`. The exact wire format is whatever the upstream ships; `m5tui-omp` is the only crate that knows it.

Inferred schema (placeholder — to be verified against OMP source):

```jsonc
// request
{ "jsonrpc": "2.0", "id": 1, "method": "session.create", "params": { "model": "minimax" } }

// response
{ "jsonrpc": "2.0", "id": 1, "result": { "session_id": "ses_abc", "model": "minimax" } }

// server event
{ "jsonrpc": "2.0", "method": "agent.event", "params": {
    "session_id": "ses_abc",
    "type": "tool_call",
    "name": "read",
    "args": { "path": "/home/pi/foo.rs" }
}}
```

`m5tui-omp` parses these and emits semantic events to the core reducer. The core doesn't see JSON; it sees "tool_call started" / "tool_call finished" / "streaming delta" / "session ended."

---

## 5. Voice subsystem

The mic path on the Cardputer-Adv is finicky because the ES8311 codec shares the I2C bus (G8/G9) with the keyboard and IMU. The `m5tui-voice` crate abstracts the audio I/O behind two traits so we can test on the Pi without a real codec.

```rust
pub trait AudioIn {
    fn start(&mut self) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
    fn samples(&mut self) -> Result<&[i16], AudioError>;
    fn sample_rate(&self) -> u32; // 16000
}

pub trait AudioOut {
    fn start(&mut self) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
    fn write(&mut self, samples: &[i16]) -> Result<(), AudioError>;
}
```

Two implementations live in the same crate:

- `cpal::AudioIn` / `cpal::AudioOut` — desktop / Pi, used in tests and `--relay` mode.
- `esp_i2s::AudioIn` / `esp_i2s::AudioOut` — Cardputer, hand-rolled using `esp-idf-sys` I2S driver + ES8311 config. This is the only place in the codebase that touches `esp-idf-sys` directly.

PTT state machine:

```
idle --(key down)--> recording --(key up)--> encoding
                                                  |
                                                  v
                                              pushing
                                                  |
                                                  v
                                              idle (with toast "pushed voice-inbox/<ts>.wav")
```

The WAV is encoded in chunks while recording so we don't hold the full sample buffer in memory. 5 minutes at 16 kHz mono i16 = ~9.6 MB; chunked encoding keeps the peak RAM low.

---

## 6. Theming deep-dive

Every visual element is rendered through a **theme context** that exposes:

```rust
pub struct ThemeCtx<'a> {
    pub palette: &'a Palette,        // 10 colors
    pub glyphs: &'a GlyphSet,         // box-drawing, scrollbar, cursor
    pub density: Density,             // Compact/Cozy/Comfy → cell size
    pub animation: AnimationLevel,    // 0..2 → effects enabled
    pub brightness: u8,               // 0..100 → backlight PWM
}
```

There is no `if theme == "cobalt"` anywhere in the codebase. Every widget takes a `&ThemeCtx` and pulls what it needs.

### 6.1 Glyph atlas

Shipped in the binary as a single `&[u8]` (8 KB), 256 glyphs at 6×8 px = 96 bytes per glyph. The font is hand-rolled to look good at 6×8 — no anti-aliasing, no subpixel. Box-drawing characters are duplicated for single/double/rounded/heavy sets, selected at theme load time.

The 6×8 framebuffer of a 40×16 grid is 240×128 px — exactly the ST7789V2 native size. **No scaling, no rotation, no surprises.**

### 6.2 Live theme editor

The editor is a stack of overlay screens, navigated with `;t`:

1. **Palette screen** — 10 rows, one per palette role. Each row shows a swatch and the current hex value. Arrow keys move the focus. `enter` opens the colour picker (16×8 grid of preset swatches plus a "type a hex" input line).
2. **Glyphs screen** — pick box-drawing set, scrollbar style, cursor style.
3. **Layout screen** — density, show/hide clock, show/hide synthwave.
4. **Animation screen** — animation level, scanline toggle, phosphor decay toggle, glitch toggle.
5. **Sound screen** — sound on/off, replace boot/click/arp WAVs from `/sd/m5tui/themes/<name>/*.wav`.
6. **Brightness screen** — slider 0-100, live updates backlight PWM.
7. **Save / Export** — `;s` saves to `/sd/m5tui/themes/<name>.yaml`; `;e` exports to stdout (you can `m5tui theme export cobalt > cobalt.yaml`).

A **live preview pane** sits in the right 16 columns of the editor at all times and re-renders the cockpit with the current draft theme every time you change anything.

---

## 7. Persistence layout

```
/m5tui/
  config.yaml                        # global config
  keys/                              # ssh private keys (chmod 600 on write)
    aiserver-1_ed25519
    desktop-ujsii52_ed25519
  profiles/
    aiserver-1.yaml
    desktop-ujsii52.yaml
    pc-ollama-direct.yaml
  themes/
    cobalt.yaml
    amber.yaml
    stealth.yaml
    mine.yaml
  history/
    aiserver-1/
      scrollback.arrow               # ring buffer
      commands.jsonl                 # every command, line-delimited
  snippets/
    aiserver-1/
      plan.yaml                      # `;plan` → "advdeck-bridge plan --project {p}"
      retry.yaml
  voice/
    2026-06-15T22-04-11Z.wav         # captured
    2026-06-15T22-05-30Z.wav
  memos/
    2026-06-15.md
  logs/
    m5tui.log                        # JSON-lines, 512 KB rotation
    m5tui.log.1                      # previous
  cache/                             # ephemeral; wiped on boot
    omrpc/                           # decoded OMP frames
    dns/                             # resolved Tailscale names
```

All YAML/JSON files carry `schema_version: 1`. Bumping is a `m5tui-persist::migrate` step on boot. Old versions are renamed to `<file>.v1.bak` and never silently overwritten.

---

## 8. Boot sequence

1. **Power on.** Cardputer-Adv boots ESP-IDF.
2. **m5tui starts** as a `std` binary under `esp-idf-sys`.
3. `m5tui-persist` mounts SD, ensures `/m5tui/` tree exists, runs migrations.
4. `m5tui-themes` loads the default theme + any user themes.
5. `m5tui-status` polls Wi-Fi state. If disconnected, shows the Wi-Fi picker screen.
6. `m5tui-profile` loads `profiles/*.yaml`. If none, the first-boot wizard runs.
7. `m5tui-ssh` opens a keepalive connection to the default profile.
8. `m5tui-omp` starts the OMP session (if `omp.enabled: true`).
9. The **cockpit** renders. You're up.

If the first-boot wizard runs, it walks: Wi-Fi creds → Tailscale login (`tailscale up` and paste the auth key) → SSH key generation (`ssh-keygen -t ed25519`, scp the public key to the aiserver) → aiserver host → default theme → mic test (record, play back).

---

## 9. Failure modes and recovery

| Failure | What the user sees | Recovery |
|---|---|---|
| SD card missing | "No SD card found. Insert and press any key." | Block, wait for key. Don't auto-create in RAM; you'll lose snippets and profiles. |
| SD card full | Toast: "SD full, freeing 1 MB" + auto-rotate logs + delete old voice memos older than 7 days | If still full, the cockpit shows a red banner. |
| Wi-Fi disconnected | Top bar: `wifi: ---`. Cockpit greys out, prompt still works locally. | Auto-reconnect on Wi-Fi state change. |
| Tailscale down | Top bar: `mesh: down`. Cockpit still works for LAN targets. | Auto-rejoin on Tailscale state change. |
| SSH handshake fails | Toast: "Connection refused / auth failed / timeout". Reconnect loop backs off. | `;d` to disconnect, `;c` to retry. |
| OMP crashes | Right pane shows last error. Session is `;restartable`. | `;r` to restart the OMP session. |
| Mic busy | `;v` shows a "mic busy, retry in 2s" toast. | Auto-retry with backoff. |
| Theme YAML invalid | Toast with the error, "using previous theme." | `;t` opens the editor, which highlights the broken line. |
| Out of memory | OOM killer hits m5tui. Cardputer reboots. Boot screen says "recovered from OOM" with a `;log` shortcut. | Logs in `/m5tui/logs/m5tui.log` show the panic. |

---

## 10. Performance budget

| Resource | Budget | Notes |
|---|---|---|
| Flash | ≤ 1.8 MB stripped | Tighter than the existing `m5-cardputer-adv` firmware because we need ratatui + russh. |
| RAM (peak) | ≤ 200 KB | Most heap is the ratatui framebuffer (~26 KB at 40×16 cells) + ring buffer (256 KB cap, configurable). |
| CPU | 240 MHz Xtensa LX7 dual core | One core for the UI render loop, one for SSH/OMP. ratatui renders in < 4 ms per frame at 30 Hz. |
| Battery | ≥ 6 hours idle, ≥ 2 hours active | Animations level 2, sound on, brightness 60%. Stealth theme pushes to 12 hours idle. |
| SSH throughput | full PTY, no throttling | russh reads at 1 MB/s easily on a typical Wi-Fi link. |
| Mic latency | < 100 ms key→samples-into-WAV | PTT feedback is instant. |

---

## 11. What this architecture explicitly rejects

- **No async-stacked-on-async-stacked-on-async.** The reducer is a plain function. Side effects are queued.
- **No global mutable state.** Everything lives in `AppState`. Every test can construct an `AppState` and feed it events.
- **No "framework" we don't own.** ratatui is the only UI dep. We don't use `cursive` or `tui-rs` or `egui-tui` — they're all dead or wrong for 40×16.
- **No `unsafe` in the cross-platform crates.** Only `m5tui-voice` (I2S driver) and `m5tui-status` (AXP2101 I2C) use `unsafe`, and only behind the trait boundary.
- **No pulling in the LLM.** This is a TUI. The LLM lives on the aiserver. End of story.

---

**End of ARCHITECTURE.md.**
