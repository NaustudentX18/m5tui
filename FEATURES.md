# m5Tui — Features

> The complete feature inventory. Each feature has a milestone tag so you
> can see at a glance what's in v1 vs. later.

Legend: `[M#]` = milestone. `P0/P1/P2` = priority within the milestone.

---

## 1. Cockpit (the home screen)  [M1, P0]

### 1.1 Layout

Three horizontal bands in a 40×16 grid:

- **Top bar** (1 row): server status · uptime · battery · signal · last sync · clock (if theme has it).
- **Main** (13 rows): left pane (agent list, 12 cols) | right pane (session state, 28 cols).
- **Bottom bar** (1 row): prompt line + status hints.
- **Toast** (overlay): 1 row, fades after 3s, on agent events.

### 1.2 Behaviour

- Boot lands on the cockpit, focus on the prompt line.
- `tab` cycles focus: prompt → agent list → session state → prompt.
- `↑/↓` in the agent list moves the selection.
- `enter` on a profile in the agent list starts a connection.
- `esc` in any pane returns focus to the prompt.

---

## 2. Command palette  [M1, P0]

A single-screen fuzzy command picker bound to `;/`.

- 40×14 modal, 2-row gap at the bottom for the search input.
- 12 built-in commands: `connect`, `disconnect`, `theme`, `voice`, `palette`, `profiles`, `handoff`, `memory`, `help`, `doctor`, `settings`, `quit`.
- User-defined commands come from snippets (see §6).
- Fuzzy match is a 60-line implementation: score = (exact match × 4) + (prefix × 2) + (subsequence × 1) + (-gap_penalty). No regex, no Levenshtein.
- `enter` runs the highlighted command, `esc` closes.

---

## 3. Theme engine + on-device editor  [M2, P0]

### 3.1 Built-in themes

| Name | Vibe | Animations | Sound |
|---|---|---|---|
| `Coldwire` | Cyberpunk cyan on midnight | 1 | on |
| `Phosphor` | Retro amber CRT | 1 | on |
| `Lacuna` | Dim red-on-black | 0 | off |
| `Magline` | Magenta/cyan grid horizon | 2 | on |
| `Noctilux` | High-contrast legibility | 0 | off |
| `Ivoryroom` | White on cream, paper feel | 0 | on |

All themes ship in the binary as embedded YAML.

### 3.2 Theme editor screens

1. **Palette** — 10 swatches, one per role. `enter` opens the colour picker.
2. **Colour picker** — 16×8 grid of preset swatches + "type a hex" line.
3. **Glyphs** — pick box-drawing set, scrollbar style, cursor style.
4. **Layout** — density, show clock, show weather (v2), show synthwave.
5. **Animation** — level (0/1/2), scanline, phosphor, glitch.
6. **Sound** — on/off, replace boot/click/arp WAVs.
7. **Brightness** — slider 0-100, live backlight update.
8. **Save** — save as new theme, or overwrite the current.
9. **Export** — print the YAML to stdout (for sharing).

The right 16 columns of every editor screen show a **live preview** of the cockpit with the draft theme applied. Every change re-renders in < 16 ms (one frame).

### 3.3 Schema validation

Every theme YAML is validated against a `jsonschema` compiled into the binary. Invalid themes are rejected with a friendly error overlay pointing at the bad line. The previous good theme stays active.

---

## 4. SSH + profiles  [M3, P0]

### 4.1 Profile registry

- `profiles/<name>.yaml` per profile.
- Fields: `host`, `address`, `address_fallback`, `port`, `user`, `identity_file`, `jump_host`, `working_dir`, `omp`, `snippets_dir`.
- Hot-reload: if the SD file mtime changes while m5Tui is running, the profile is reloaded on next focus.
- `;p` opens a profile picker; `;n` creates a new profile (first-boot wizard for new profiles).

### 4.2 First-boot wizard

A 6-screen wizard:

1. **Wi-Fi** — scan, pick a network, enter the password (PSK or EAP if supported).
2. **Tailscale** — paste the auth key, choose the hostname, `tailscale up`.
3. **SSH key** — generate an ed25519 keypair on the device, show the public key, "press a key when you've added this to `~/.ssh/authorized_keys` on the server."
4. **Server** — pick from a list of known Tailscale peers (if Tailscale is up) or type the address.
5. **Theme** — pick a theme.
6. **Mic test** — record 3s, play back, "did you hear it?" y/n.

### 4.3 Connection management

- `;c` opens a small "connect to…" picker (any profile in the registry).
- `enter` connects, `;d` disconnects, `;r` reconnects.
- Keepalive pings every 15s; 3 missed pings → reconnect with backoff.
- Connection health shows in the top bar: `aiserver-1 :: OK` / `:: RECONNECTING (3/5)` / `:: DOWN`.

### 4.4 Inline terminal pane

- Anything you type that isn't `;`-prefixed is sent to the remote PTY.
- Output is rendered through a less-style pager: `j/k` line, `g/G` top/bottom, `/` search, `n/N` next/prev match.
- Scrollback is captured client-side, capped at 256 KB, persisted to `/m5tui/history/<host>/scrollback.arrow`.
- The scrollback is searchable across reboots (grep-able JSONL fallback if the binary format is broken).

### 4.5 SCP

- `;wav` lists local voice memos; pick one, scp to `~/voice-inbox/`.
- `;fetch` opens an SFTP browser on the remote.
- `;up` / `;down` transfer the highlighted file.

---

## 5. OMP integration  [M4, P0]

### 5.1 Connection model

- `omp --mode rpc` runs in a dedicated exec channel.
- m5Tui sends JSON-RPC 2.0 frames; receives JSON-RPC frames + server-sent events.
- Auto-reconnect on protocol errors with backoff.

### 5.2 Rendered widgets

| OMP event | Rendered as |
|---|---|
| `agent.text_delta` | Streaming text in the right pane, monospace, themed. |
| `agent.tool_call.start` | A card: `▶ read /home/pi/foo.rs` (tool name + args). |
| `agent.tool_call.end` | The card collapses to `✓ read /home/pi/foo.rs (0.4s)`. |
| `agent.todo_update` | A live todo list in the right pane. |
| `agent.subagent` | A nested subagent tree in the right pane. |
| `agent.error` | A red toast for 5s. |
| `session.usage` | Updates the `ctx 12K/512K` line. |
| `session.thinking` | The right pane gets a "thinking…" badge with a tiny spinner glyph. |

### 5.3 User input

- The prompt line accepts a single-line message; `enter` sends it to OMP.
- `shift+enter` inserts a newline for multi-line messages.
- `;ask <text>` is a shortcut for "type into the prompt and send immediately."
- `;continue` continues the previous session.

### 5.4 Profile switching

- `;a` opens the profile picker; selecting a new one stops the current OMP session, swaps the SSH connection, and starts a new OMP session.
- The previous session is checkpointed (a `omp --continue` flag) so you can come back later.

---

## 6. Snippets / command expansion  [M3, P1]

Snippets are per-profile text expanders.

```yaml
# snippets/aiserver-1/plan.yaml
schema_version: 1
triggers:
  - ";plan"           # in command palette
  - "/plan "          # in prompt
template: |
  advdeck-bridge plan --project {project} --storage-root /advdeck
prompts:
  - name: project
    message: "project slug?"
```

`/plan garden` expands to `advdeck-bridge plan --project garden --storage-root /advdeck` after prompting for `{project}`.

Shipped snippets in v1:

| Trigger | Expands to |
|---|---|
| `;plan` | `advdeck-bridge plan --project {project} --storage-root /advdeck` |
| `;retry` | `advdeck-bridge retry {request_id}` |
| `;export` | `advdeck-bridge export --project {project} --out ./export` |
| `;export-issues` | `advdeck-bridge export --project {project} --out ./issues --format github-issues` |
| `;git-status` | `git -C /home/pi/{project} status` |
| `;git-log` | `git -C /home/pi/{project} log --oneline -20` |
| `;restart-omp` | `pkill -f 'omp --mode rpc' && omp --mode rpc &` |
| `;tailscale-status` | `tailscale status` |
| `;ollama-ps` | `curl -s http://desktop-ujsii52.local:11434/api/ps` |
| `;mem-search` | `obsidian-memory search "{query}"` |

---

## 7. Voice / push-to-talk  [M5, P0]

### 7.1 PTT control

- `;v` (hold) — record. `;v` (tap) — open the voice menu.
- The voice menu: `record`, `play last`, `delete last`, `push last`, `auto-push toggle`.
- Recording is shown as a 1-bar VU meter in the bottom bar.
- On release, the WAV is saved to `/m5tui/voice/<iso-ts>.wav`.

### 7.2 Auto-push

- If `voice.auto_push: true` in the config, the WAV is `scp`'d to `~/voice-inbox/` on the default profile.
- The toast says `pushed ~/voice-inbox/2026-06-15T22-04-11Z.wav`.
- If `voice.auto_enqueue_plan: true`, an `advdeck-bridge plan` job is enqueued with `transcript.md` set to "see voice-inbox/<file>.wav" (the bridge will pick it up and transcribe).

### 7.3 Live transcript overlay  [M5, P2 — stretch]

- If `voice.stream_stt_url` is set (e.g. `http://desktop-ujsii52:8080/stt`), the device streams mic frames over the wire to an STT sidecar.
- The transcript appears under the prompt line in real time.
- Requires a sidecar not in v1; the field exists so v1.1 can add it without breaking the schema.

### 7.4 Playback

- `;p` plays the last voice memo through the ES8311 → NS4150B speaker.
- Volume is the themable slider.
- File picker: `;voice-list` shows all voice memos with sizes; `enter` to play.

---

## 8. Handoff viewer  [M6, P1]

- `;h` opens the handoff picker.
- The picker lists every project under `/home/pi/projects/` (or whatever the config says) that has an `agent-prompt.md`.
- `enter` reads the file via SFTP, renders it as Markdown (using a small inline renderer, no external lib).
- `;continue` starts a new OMP session with the agent prompt as the system prompt prefix.
- `;save` saves the handoff locally to `/m5tui/memos/<date>.md`.

---

## 9. Memory / vault search  [M6, P1]

- `;m` opens the search box.
- The box calls `obsidian-memory search "{query}"` over SSH and renders the JSONL output as a hit list.
- `enter` opens the hit in a Markdown viewer.
- Recent searches are persisted to `/m5tui/memos/search-history.jsonl`.

---

## 10. Status + doctor  [M7, P0]

### 10.1 Top-bar status

- **Server** — `OK` / `RECONNECTING n/5` / `DOWN`. Driven by SSH keepalive.
- **Uptime** — `4h12m`. From `/proc/uptime` on the server, polled every 30s.
- **Battery** — `73% ⚡` (charging glyph if charging). AXP2101 I2C poll.
- **Signal** — `▁▃▆█` (RSSI bar glyph). ESP32 Wi-Fi API.
- **Last sync** — `2m ago`. Time since last successful keepalive ping.
- **Clock** — if the theme has `show_clock: true`, the local time. NTP-synced.

### 10.2 Doctor

- `;D` runs `m5tui doctor`, a full self-check.
- Returns a green/yellow/red scoreboard:
  - Wi-Fi: connected? RSSI? IP?
  - Tailscale: mesh up? MagicDNS working?
  - SSH: handshake OK to default profile? Key accepted? Banner?
  - aiserver: disk space > 1 GB? OMP responds? advdeck-bridge present?
  - OMP: RPC schema version matches? Last request < 60s old?
  - Mic: capture test passes? Playback works?
  - SD: free space? R/W? Atomic write test?
- The scoreboard is rendered as a 40×16 screen with colour-coded rows. `;e` exports it to `/m5tui/logs/doctor-<iso-ts>.md` for sharing.

---

## 11. Hardware integration  [M7, P0]

### 11.1 IMU

- Double-tap on the Cardputer (BMI270 interrupt) wakes the screen from off. Configurable in `settings → imu → wake_on_double_tap`.
- Shake-to-cancel: shake the Cardputer while a command is running → cancel the command. Off by default.

### 11.2 IR

- `;i` flashes the current theme's accent colour through the IR emitter (just on/off at 38 kHz for 200 ms — no protocol, novelty only).
- v2 may add a "learn IR remote codes" feature.

### 11.3 Backlight

- Brightness slider in the theme editor drives G38 PWM via the AXP2101 backlight channel.
- Auto-dim on idle (configurable timeout, default 60s) and on battery below 20% (default).

### 11.4 Battery alerts

- Toast: "Battery low (15%)" at 15%.
- Toast: "Critical (5%) — auto-saving session" at 5%. m5Tui flushes all pending writes and shows a "please charge" banner.

---

## 12. Help system  [M1, P0]

- `;?` opens the **hotkey overlay** — a full-screen modal listing every binding, grouped by pane.
- `;?` from the overlay opens the **about screen** — version, build time, SD layout, theme, profile, uptime, battery.
- `;L` opens the **log viewer** — tail of `/m5tui/logs/m5tui.log` with `j/k/g/G` and `/` search.

---

## 13. Boot screen  [M8, P1]

- 40×16 ASCII rendering of the m5Tui logo with a fake "loading" animation.
- Skippable with `enter`.
- The boot screen plays the theme's "boot" sound (one of the WAVs).
- "Loading" stages: `init sd` → `load themes` → `wifi connect` → `tailscale mesh` → `ssh handshake` → `omp handshake` → `done`.

---

## 14. Sound design  [M8, P1]

Every theme has 3 WAV files (max 4 KB each, 8 kHz mono PCM):

- `boot.wav` — played once on boot.
- `click.wav` — played on every key (throttled to 50 ms to avoid noise on rapid typing).
- `arp.wav` — played on a successful agent handoff or a successful SSH connect.

`m5tui-sound` CLI can record/replace them:

```bash
m5tui-sound record click      # record from the on-board mic
m5tui-sound import click ~/  # import a WAV from a path
m5tui-sound silence          # mute the current theme's sounds
```

---

## 15. Synthwave horizon  [M8, P2]

A 40×2 perspective grid rendered in the bottom bar (if the theme has `synthwave: true`).

```
v 0.1.0 ◆ aiserver-1 :: OK ◆ 73% ▁▃▆
═══════════════════════════════════════
    ╲   │   ╱   │   ╲   │   ╱   │
     ╲  │  ╱    │    ╲  │  ╱    │
      ╲ │ ╱     │     ╲ │ ╱     │
       ╲│╱      │      ╲│╱      │
        V       │       V       │
─────────────────────────────────────
```

Pure decoration. Looks incredible. Turns off automatically in `stealth` and `mono` themes.

---

## 16. Future (v2+)

| Feature | Why it's not v1 |
|---|---|
| Companion web UI (SoftAP) | Loses the keyboard-first feel. v2 might be a *parallel* surface, not a replacement. |
| BLE HID emulation | Pairing is fragile; Cardputer BLE stack is the worst part of the SDK. |
| LoRa / GPS / RFID | Out of MVP brief; possible companion hardware projects. |
| Multi-user auth | This is a personal device. |
| Plugin / extension system | Adds attack surface; defer until v1 is stable. |
| Custom widget DSL | YAGNI until we have a real second widget author. |
| Local LLM fallback | The Cardputer can't run any useful LLM. Defer until the hardware changes. |
| Voice wake word ("Hey m5tui") | Always-on mic drains battery. v2 with the Cardputer speaker replaced. |
| Cloud sync of themes | Privacy concern; defer. |

---

**End of FEATURES.md.**
