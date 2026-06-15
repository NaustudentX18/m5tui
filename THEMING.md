# m5Tui — Theming & On-Device Personalisation

> The "sexy and fun" brief, made concrete. This is the most important UX
> doc in the project — the entire visual identity of m5Tui lives here.

---

## 1. Design philosophy

> **Every pixel is themable. The user owns their environment. The name of a theme should describe what it looks like.**

The Cardputer is a personal device. The person using it is the person
looking at it. If the default theme doesn't fit, the user changes it
*on the device* in 30 seconds, saves it, and never thinks about it again.

Three rules:

1. **The theme is data, not code.** Every visual decision lives in a
   `theme.yaml` file. There is no `if theme == "coldwire"` in the codebase.
   If we discover we need a per-theme code path, that's a code smell —
   push the decision into the data.
2. **The default theme is opinionated.** The `Coldwire` theme is the
   hero theme; it sets the visual standard. Other themes are variations.
3. **Live preview is non-negotiable.** Every change in the theme editor
   re-renders the cockpit *right now*. The user never has to "save and
   reload" to see what their change looks like.
4. **The name is the look.** A theme's name should make you see it.
   `Coldwire` (cold + wire, a thin blue line through frozen data) is
   instantly readable. `Lacuna` (a gap in the light) is instantly
   readable. `Phosphor` (the glow on the back of an old CRT) is
   instantly readable. We never use names that require explanation.

---

## 2. The six pre-installed themes

The names read like the look:

| Name | Vibe | What you see when you read the name |
|---|---|---|
| `Coldwire` | Cyberpunk cyan on midnight | cold + wire = a thin blue line through a frozen data stream |
| `Phosphor` | Amber CRT, vt220 vibes | phosphor = the glow on the back of an old CRT |
| `Lacuna` | Dim red-on-black, almost off | lacuna = a gap, a missing piece, a hole in the light |
| `Magline` | Magenta on indigo, synthwave grid | magenta + line = a horizon line of pink light |
| `Noctilux` | High-contrast, legibility-first | noctilux = "night light" — the brightest thing in the dark |
| `Ivoryroom` | White on cream, paper-notebook feel | an ivory room = quiet, blank, nothing hidden |

All six ship in the binary as embedded YAML.

### 2.1 `Coldwire` — the hero theme

This is what the user sees on first boot. Every other theme is a remix.

**Palette:**

```
bg        #0a1428    deep midnight blue
fg        #d0e0ff    cool off-white
accent    #00d8ff    electric cyan
dim       #506a8a    muted blue-grey
warn      #ffaa00    amber alert
err       #ff3050    hot red
ok        #44ee88    mint green
sel_bg    #1a3868    focused-row background
sel_fg    #ffffff    focused-row foreground
prompt    #00d8ff    matches accent (the cursor line)
```

The accent (`#00d8ff`) is the only colour that "moves" — it pulses, it
glitches, it's the colour of the prompt cursor. Everything else is
static.

**Glyphs:**

- Box drawing: `heavy` (━━━ ┃┃ ┏━┓ etc.) for the main frame.
- Scrollbar: `block` (█▌) for the pager.
- Cursor: `block` (█) for the prompt, `underline` (▁) in input fields.
- Sparklines: built from `▁▂▃▄▅▆▇█` half-blocks.
- Progress: built from `▏▎▍▌▋▊▉█` eighth-blocks.

**Density:** `compact` — 40 cols × 16 rows. The Cardputer's native grid.

**Animations:** Level 1 — cursor pulse, glitch on event, scanline, synthwave horizon.

**Sound:** On. Click on every key (throttled 50 ms). Boot chime on power-on.
Ascending arpeggio on successful agent handoff.

**Brightness:** 60% (drives G38 backlight PWM via the AXP2101 PMIC).

### 2.2 `Phosphor` — the retro CRT theme

> "Amber phosphor, like a VT220."

```yaml
name: phosphor
palette:
  bg:     "#1a0e02"
  fg:     "#ffb000"
  accent: "#ffd060"
  dim:    "#804000"
  warn:   "#ff7000"
  err:    "#ff2020"
  ok:     "#ffd060"
  sel_bg: "#3a1e02"
  sel_fg: "#ffffff"
  prompt: "#ffd060"
glyphs:
  box: ascii               # VT220-style ASCII boxes
animation:
  level: 1
  scanline: true
  phosphor_decay: true     # amber phosphor fade
```

### 2.3 `Lacuna` — the dim red theme

> "A gap in the light. Almost off."

```yaml
name: lacuna
palette:
  bg:     "#000000"
  fg:     "#300000"
  accent: "#ff0030"
  dim:    "#200000"
  warn:   "#ff5000"
  err:    "#ff0030"
  ok:     "#ff0030"
  sel_bg: "#1a0000"
  sel_fg: "#ff5050"
  prompt: "#ff0030"
glyphs:
  box: rounded
animation:
  level: 0                 # off
  scanline: false
  phosphor_decay: false
sound:
  enabled: false
brightness: 30             # dim by default
```

### 2.4 `Magline` — the synthwave theme

> "A horizon line of pink light."

```yaml
name: magline
palette:
  bg:     "#0a0028"
  fg:     "#f0d0ff"
  accent: "#ff00aa"
  dim:    "#5028a0"
  warn:   "#ffaa00"
  err:    "#ff3050"
  ok:     "#00ffaa"
  sel_bg: "#2a0050"
  sel_fg: "#ffffff"
  prompt: "#00d8ff"
glyphs:
  box: double
animation:
  level: 2                 # full cyberpunk
  scanline: true
  phosphor_decay: true
  glitch_on_event: true
  synthwave: true
```

### 2.5 `Noctilux` — the legibility theme

> "Night light. The brightest thing in the dark."

```yaml
name: noctilux
palette:
  bg:     "#000000"
  fg:     "#ffff00"
  accent: "#ffffff"
  dim:    "#aaaa00"
  warn:   "#ff8000"
  err:    "#ff0000"
  ok:     "#00ff00"
  sel_bg: "#ffffff"
  sel_fg: "#000000"
  prompt: "#ffff00"
glyphs:
  box: heavy
layout:
  density: comfy            # bigger cells
animation:
  level: 0
sound:
  enabled: false
```

### 2.6 `Ivoryroom` — the paper theme

> "A quiet room, blank, nothing hidden."

```yaml
name: ivoryroom
palette:
  bg:     "#f4ecd8"         # cream
  fg:     "#1a1a1a"         # near-black
  accent: "#a04830"         # burnt sienna
  dim:    "#8a7a5a"         # warm grey
  warn:   "#b86000"
  err:    "#a01010"
  ok:     "#308a30"
  sel_bg: "#d8c8a0"
  sel_fg: "#000000"
  prompt: "#a04830"
glyphs:
  box: single
layout:
  density: comfy
animation:
  level: 0
  scanline: false
sound:
  enabled: true             # paper rustle, not clicks
  click: /sd/m5tui/themes/ivoryroom/click.wav
  arp:   /sd/m5tui/themes/ivoryroom/arp.wav
```

---

## 3. The theme YAML schema

```yaml
schema_version: 1          # required; bump + add a migration when changing
name: coldwire             # required, must match the filename
author: forest             # optional, shown in the Market

# --- Palette ----------------------------------------------------------
palette:
  bg:        "#0a1428"     # default background
  fg:        "#d0e0ff"     # default foreground (text)
  accent:    "#00d8ff"     # accent (the "alive" colour)
  dim:       "#506a8a"     # dimmed text (timestamps, hints)
  warn:      "#ffaa00"     # warnings, low battery
  err:       "#ff3050"     # errors
  ok:        "#44ee88"     # success, OK state
  sel_bg:    "#1a3868"     # focused row background
  sel_fg:    "#ffffff"     # focused row foreground
  prompt:    "#00d8ff"     # prompt cursor colour

# --- Glyphs -----------------------------------------------------------
glyphs:
  box: heavy                # ascii | single | double | rounded | heavy
  scrollbar: block          # block | arrow | line
  cursor: block             # block | underline | bar

# --- Layout -----------------------------------------------------------
layout:
  density: compact          # compact | cozy | comfy
  show_clock: false         # show HH:MM in the top bar
  show_synthwave: true      # render the synthwave horizon
  show_battery_icon: true   # show a battery glyph next to the %

# --- Animation --------------------------------------------------------
animation:
  level: 1                  # 0..2 (0 = none, 2 = full cyberpunk)
  scanline: true            # 1px alternating dim overlay
  phosphor_decay: false     # text fades to dim over 400ms
  glitch_on_event: true     # 200ms title bar distortion on tool calls
  synthwave: true           # perspective grid in the bottom bar
  cursor_pulse: true        # 300ms cursor pulse

# --- Sound ------------------------------------------------------------
sound:
  enabled: true
  boot:  /sd/m5tui/themes/coldwire/boot.wav
  click: /sd/m5tui/themes/coldwire/click.wav
  arp:   /sd/m5tui/themes/coldwire/arp.wav
  throttle_ms: 50           # min ms between click sounds

# --- Brightness -------------------------------------------------------
brightness: 60              # 0..100, drives G38 backlight PWM

# --- Market metadata --------------------------------------------------
market:
  tested_omp_versions: ["15.13.3"]
  tags: ["cyberpunk", "cyan", "synthwave"]
```

Every field has a default. A theme with no `palette:` block is the
`Noctilux` theme. A theme with no `animation:` block has animations
off. This means a user can hand-edit a theme to be minimal without
breaking anything.

---

## 4. The theme editor

Bound to `;t` from anywhere. A stack of overlay screens, navigated with
`;` + number, or with the on-screen tab bar.

### 4.1 Palette screen (the screen you'll spend the most time on)

```
┌─THEME: mine ────PALETTE──────────────────┐┌─LIVE PREVIEW──┐
│                                          ││ aiserver-1    │
│ > bg         ▓▓▓▓ #0a1428  ▒▒▒▒          ││ :: OK         │
│   fg         ▓▓▓▓ #d0e0ff  ▒▒▒▒          ││ 73%           │
│   accent     ▓▓▓▓ #00d8ff  ▒▒▒▒          ││               │
│   dim        ▓▓▓▓ #506a8a  ▒▒▒▒          ││ AGENTS  SESS  │
│   warn       ▓▓▓▓ #ffaa00  ▒▒▒▒          ││ > srv  14/22  │
│   err        ▓▓▓▓ #ff3050  ▒▒▒▒          ││       ctx 12K │
│   ok         ▓▓▓▓ #44ee88  ▒▒▒▒          ││       31 t/s  │
│   sel_bg     ▓▓▓▓ #1a3868  ▒▒▒▒          ││               │
│   sel_fg     ▓▓▓▓ #ffffff  ▒▒▒▒          ││ ▁▂▃▄▅▆▇█      │
│   prompt     ▓▓▓▓ #00d8ff  ▒▒▒▒          ││ > _           │
│                                          ││               │
│  ;p palette ;g glyphs ;l layout          ││               │
│  ;a anim    ;s sound  ;b brightness      ││               │
│  ;w save    ;e export ;q back            ││               │
└──────────────────────────────────────────┘└───────────────┘
```

The left half is the editor; the right half is a live 16-col preview of
the cockpit with the draft theme applied. Every arrow key press or
colour change re-renders the right half in < 16 ms.

### 4.2 Colour picker

`enter` on a swatch opens the colour picker:

```
┌─PICKER: accent─────────────────────────────┐
│   ░░░ ▒▒▒ ▓▓▓ ███ ░██ ▒██ ▓██ ██░ ██▒    │
│   ░░▒ ▒▒▓ ▓▓█ ██▒ ██▓ ██▒ ▒██ ▓█▒ ██     │
│   ░░█ ▒█▒ ▓█▒ ██▓ ██▒ ██░ ░██ ▒█▓ █▒     │
│                                            │
│   # hex: 00d8ff_                           │
│                                            │
│   ;enter apply  ;esc cancel                │
└────────────────────────────────────────────┘
```

A 16×8 grid of preset swatches. The hex input line lets the user type
a custom colour. `enter` applies; the palette screen redraws.

### 4.3 Brightness slider

A horizontal bar:

```
BRIGHTNESS: ━━━━━━━━━━░░░░░░░░░░  50%
```

`←/→` adjusts by 5, `shift+←/→` by 1. The slider drives the G38 PWM
*live* — the user sees the screen get brighter or dimmer as they drag.

### 4.4 Save / Export / Publish

- `;w` opens a "save as…" prompt. Default name is `mine`. Overwrite
  confirmation if the name exists.
- `;e` exports the current draft to stdout as YAML. Pair with
  `m5tui theme import < /path/to/theme.yaml` on the same or another
  device to share themes.
- `;m p` publishes to the Community Theme Market. See `MARKET.md`.

---

## 5. Animations — what they actually do

### 5.1 Cursor pulse

A 300 ms sine wave on the prompt cursor's `bg` channel, between
`prompt` and `prompt × 0.7`. Subtle, low cost. Off at animation level 0.

### 5.2 Glitch on event

When the OMP RPC stream emits a `tool_call.start` or `session.usage`
update, the title bar (1 row) is re-rendered with a small horizontal
offset for 200 ms (4 frames at 30 Hz). The cost is negligible — the
title bar is 40 cells.

### 5.3 Scanline overlay

Every other line of the framebuffer has its `bg` colour darkened by
20%. This is a single multiplication per cell. Cost: 1 mul per render.

### 5.4 Phosphor decay

When new text appears in the right pane, the previous text fades from
`fg` to `dim` over 400 ms (24 frames at 60 Hz — but we render at 30 Hz,
so it's 12 frames). The cost is a per-cell colour interpolation on the
fading lines only.

### 5.5 Synthwave horizon

A 40×2 perspective grid in the bottom bar. Recomputed every frame from
a phase variable that increments at 4 Hz. Cost: 80 sin/cos per frame.

### 5.6 Animation level

| Level | Cursor pulse | Glitch | Scanline | Phosphor | Synthwave |
|------:|:-----------:|:------:|:--------:|:--------:|:---------:|
| 0     |             |        |          |          |           |
| 1     | ✓           | ✓      | ✓        |          | ✓         |
| 2     | ✓           | ✓      | ✓        | ✓        | ✓         |

`Lacuna`, `Noctilux`, and `Ivoryroom` set `level: 0` by default.
`Magline` sets `level: 2`.

---

## 6. Sound design

Three WAVs per theme, each ≤ 4 KB, 8 kHz mono PCM.

| File | When | Default coldwire content |
|---|---|---|
| `boot.wav` | Power-on, once | A 0.4 s 440 Hz → 880 Hz sweep |
| `click.wav` | Every key (throttled 50 ms) | A 30 ms square wave at 2 kHz |
| `arp.wav` | Successful connect / handoff | C-E-G arpeggio, 100 ms each |

The user can replace these with their own recordings. `m5tui-sound` CLI
is a separate binary shipped alongside `m5tui` for headless recording
and import.

Themes that ship with sound off (`Lacuna`, `Noctilux`) are silent. The
binary defaults are procedural (generated at build time); themes can
override with their own WAVs in the theme folder.

---

## 7. Sharing themes

A theme is one YAML file plus (optionally) three small WAVs. To share a
theme:

### 7.1 Via the Market (preferred)

`;mp` (or `;m p` then "publish my current theme"). See `MARKET.md` for
the full flow.

### 7.2 Via a file (offline)

```bash
# On the device
;e          # exports the current theme to stdout
```

```bash
# On a host with the file
adb pull /sd/m5tui/themes/mine.yaml .
# Share it: send the YAML in chat, post it on GitHub, etc.
```

To install a shared theme on a device:

```bash
# From the device, with the YAML on a path
m5tui theme import < ~/Downloads/coldwire-mine.yaml

# Or copy to SD and reboot
cp coldwire-mine.yaml /sd/m5tui/themes/
```

The theme gallery in the GitHub repo will host community themes. The
v1 repo ships with the 6 pre-installed themes + 4 community themes
once the Market is live.

---

## 8. Open questions for the SWAT

1. **Default hero theme.** Is `Coldwire` the right hero, or do you
   want `Phosphor` (amber) to lead and `Coldwire` as the alternate?
2. **Animations on by default.** Is animation level 1 the right
   default for the hero theme, or should we ship animation level 0
   as the default and let users opt up?
3. **Sound on by default.** `Coldwire`, `Phosphor`, `Magline`, and
   `Ivoryroom` are on; `Lacuna` and `Noctilux` are off. OK?
4. **Six themes enough.** v1 ships 6. v1.1 can add more. OK to defer
   extras to the Market from day one?
5. **`Ivoryroom`** is a paper theme. The brief was "sexy and fun" —
   does a paper theme belong in the pre-installed set, or should it
   be a community example?

---

**End of THEMING.md.**
