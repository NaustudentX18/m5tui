# m5Tui — Hardware Integration

> Every pin, every bus, every driver. This is the spec an embedded
> engineer needs to write the `m5tui-voice` and `m5tui-status` crates
> against. Most of it is the same as the existing `m5-cardputer-adv`
> firmware (`/home/pi/m5-cardputer-adv/research/hardware/cardputer-adv-hardware.md`)
> — we don't reinvent drivers, we wrap them.

Target device: **M5Stack Cardputer-Adv (SKU K132-Adv)**, ESP32-S3FN8,
Stamp-S3A core, 8 MB flash, no PSRAM.

---

## 1. Bill of materials recap

| Subsystem | Chip | Bus | Pins | Notes |
|---|---|---|---|---|
| Display | ST7789V2 | SPI | MOSI G35, SCK G36, CS G37, DC G34, RST G33, BL G38 | 240×135, 16-bit colour |
| Keyboard | TCA8418RTWR | I2C | SDA G8, SCL G9, INT G11 | 4×14 matrix, 56 keys |
| Audio codec | ES8311 | I2C + I2S | I2C: SDA G8, SCL G9. I2S: BCLK G41, DOUT G46, LRCK G43, DIN G42 | Mono mic in, mono speaker out |
| Speaker amp | NS4150B | analogue | (driven by ES8311 line out) | 8Ω / 1 W |
| Microphone | MEMS | analogue → ES8311 | (on codec) | 65 dB SNR |
| IMU | BMI270 | I2C | SDA G8, SCL G9, INT1 G4 | 6-axis |
| IR TX | (LED + driver) | GPIO | G44 | 38 kHz carrier |
| microSD | (SPI) | SPI | CS G12, MOSI G14, MISO G39, CLK G40 |  |
| PMIC | AXP2101 | I2C | SDA G8, SCL G9, IRQ G3 | Battery, charging, backlight |
| Wi-Fi | ESP32-S3 radio | — | (internal) | 2.4 GHz, 802.11 b/g/n |
| Boot | GPIO0 | — | G0 | Hold low at power-on → download mode |

**Gotcha:** G8/G9 are busy. I2C is shared between keyboard, ES8311, BMI270, and AXP2101. The driver layer must serialise I2C transactions to avoid bus contention. The existing `m5-cardputer-adv` firmware has a `i2c_mutex` we can reference.

---

## 2. Display (ST7789V2)

### 2.1 Init sequence

```c
// standard ST7789V2 240x135 init, identical to m5-cardputer-adv
static const uint8_t init_cmds[] = {
    0x01, 0,             // SWRESET
    0x11, 0,             // SLPOUT
    0x3A, 1, 0x55,       // COLMOD: 16-bit RGB
    0x36, 1, 0x60,       // MADCTL: row/col swap for landscape
    0x21, 0,             // INVON
    0x29, 0,             // DISPON
    0x00                 // sentinel
};
```

### 2.2 Framebuffer model

The m5Tui TUI core never talks to the ST7789V2 directly. It produces a
`Frame { cells: [[Cell; 40]; 16] }` where `Cell { glyph: u8, fg: u16, bg: u16, attrs: u8 }`.

A separate render task converts each frame to a 240×135 RGB565 buffer
and pushes it to the display via SPI DMA:

```
Frame (40x16 cells, ~13 KB)
  -> cell-to-pixel conversion (one 6x8 glyph per cell, ~30 KB output)
  -> RGB565 framebuffer (240x135 * 2 = 64.8 KB)
  -> SPI DMA transfer (~3 ms at 40 MHz)
```

The cell-to-pixel converter is also the simulator backend: it produces
a `Vec<u8>` of RGB565 bytes that the unit tests can golden-diff.

### 2.3 Backlight

The AXP2101 drives the backlight via DCDC3 (or DLDO2 — needs verification
against the actual ADV schematic). m5Tui sets the duty cycle via the
AXP2101 I2C register, with a default of 60% and a slider in the theme
editor.

### 2.4 Scanline + phosphor

The cell-to-pixel converter is the only place that implements scanline
(darken every other line by 20%) and phosphor decay (interpolate
fading text to `dim`). These are pure functions of the cell colour, so
they cost almost nothing.

---

## 3. Keyboard (TCA8418 over I2C + GPIO interrupt)

### 3.1 Matrix

56 keys in a 4×14 grid. The exact keymap (from the existing firmware
reference):

```
q w e r t y u i o p [ ] \
a s d f g h j k l ; ' del
   z x c v b n m , . /
        (space)
```

(more precisely: 4 rows of 14 keys each, with one key acting as the
"Fn" modifier in the bottom-left position)

### 3.2 Hotkey map

m5Tui's hotkeys live on the symbol layer (anywhere a `;` can prefix
them). The single-key hotkeys are:

| Key | Action |
|---|---|
| `;` | Hotkey prefix (combinator) |
| `?` | Help overlay |
| `/` | Command palette (also reachable as `;/` etc.) |
| `tab` | Cycle focus |
| `esc` | Cancel / back |
| `enter` | Confirm / send |
| `shift+enter` | Newline (in prompt) |
| `;v` | Voice PTT (hold) |
| `;t` | Theme editor |
| `;p` | Profiles picker |
| `;c` | Connect |
| `;d` | Disconnect |
| `;r` | Reconnect |
| `;a` | Agent / OMP profile switch |
| `;h` | Handoff viewer |
| `;m` | Memory search |
| `;D` | Doctor |
| `;L` | Log viewer |
| `;i` | IR flash (novelty) |
| `;?` | About |

Modifier chords that need to be cheap:

- `shift+↑/↓/←/→` — same as arrows (shift is the only modifier on the
  Cardputer that's "easy" — the others are Fn + key).
- `Fn+letter` — reserved for the on-device shell escape hatch
  (e.g. `Fn+q` to suspend, `Fn+x` to quit). Not used by m5Tui directly.

### 3.3 Driver

The TCA8418 exposes a key-event FIFO over I2C. The driver:

1. Configures the matrix to 4 rows × 14 cols, debounce 5 ms.
2. Enables the GPIO interrupt on G11 (active low).
3. On interrupt, drains the FIFO and pushes `KeyEvent` to the input
   channel.
4. The input channel maps (row, col) to a `Key` enum and feeds it to
   the reducer.

The existing `m5-cardputer-adv` firmware has a working `M5Cardputer`
driver. We port the keymap, not the matrix scan.

---

## 4. Audio (ES8311 over I2C + I2S)

### 4.1 Init sequence

```c
// ES8311 init, identical to m5-cardputer-adv voice-memo path
es8311_soft_reset();
es8311_set_mode(ES8311_MODE_SLAVE);
es8311_set_clock(...);  // 16 kHz sample rate, 256x MCLK
es8311_set_dac(...);    // 16-bit, mono
es8311_set_adc(...);    // 16-bit, mono, +24 dB PGA
es8311_set_speaker_output(true);
```

### 4.2 Mic capture path

```
MEMS mic ──> ES8311 ADC ──> I2S RX (DIN=G42)
                                       │
                                       v
                              I2S DMA buffer (4 KB ring)
                                       │
                                       v
                              WAV encoder (chunked)
                                       │
                                       v
                              /m5tui/voice/<ts>.wav
```

Sample rate: 16 kHz. Bit depth: 16-bit. Channels: 1 (mono). Bitrate:
256 kbps. 5 minutes of audio = 9.6 MB; we write chunks of 4 KB to keep
the peak RAM low.

### 4.3 Speaker playback path

```
/m5tui/voice/<ts>.wav
       │
       v
WAV decoder (chunked)
       │
       v
I2S TX (DOUT=G46) ──> ES8311 DAC ──> NS4150B ──> 8Ω speaker
```

Volume is controlled by the ES8311 DAC attenuator (or the NS4150B gain
pin, depending on schematic — needs verification). Default 70%.

### 4.4 Latency budget

- Key down → first samples in the WAV: < 100 ms.
- Key up → file flushed and saved: < 50 ms.
- WAV scp'd to aiserver: < 500 ms for a 1 MB file over Tailscale.

---

## 5. IMU (BMI270)

### 5.1 Use cases

- **Wake on double-tap.** A double-tap (≥ 2g acceleration within 200 ms
  in any axis) wakes the screen from off. Configurable in settings.
- **Shake to cancel.** A shake (≥ 3g in 3 axes within 50 ms) cancels
  the currently running OMP request. Off by default.
- **Orientation hint.** A reading of "screen-up" auto-raises the
  brightness. Off by default.

### 5.2 Driver

The BMI270 is on the shared I2C bus. Polling at 100 Hz is acceptable
but the existing firmware uses interrupt-driven reads. We do the same.

---

## 6. IR (TX only)

The Cardputer-Adv has one IR emitter on G44.

- **Novelty mode:** `;i` flashes the current theme's accent colour.
  The "flash" is 200 ms of 38 kHz carrier, no protocol, just a visual
  "look, the room is glowing cyan."
- **No RX.** The Cardputer has no IR receiver. Receiving IR codes is
  a v2 feature with an external sensor.

The driver is one GPIO + one timer for the 38 kHz carrier. Trivial.

---

## 7. microSD

### 7.1 Layout

The default m5Tui root is `/sd/m5tui/`. The full layout is in
`PLANNING.md §5.5` and `ARCHITECTURE.md §7`.

### 7.2 Atomic writes

All writes to YAML/JSON files are atomic:

```rust
fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
```

`fsync` is called before the rename on platforms that support it. On
ESP32, the underlying `littlefs` or `fatfs` driver handles durability.

### 7.3 Log rotation

`/m5tui/logs/m5tui.log` is rotated at 512 KB:

```
m5tui.log          (current, 0-512 KB)
m5tui.log.1        (previous, 0-512 KB)
m5tui.log.2        (older, 0-512 KB, then deleted)
```

Rotation is in-process: when the current file crosses 512 KB, it's
closed, `m5tui.log.1` is renamed to `m5tui.log.2`, the current is
renamed to `m5tui.log.1`, and a new `m5tui.log` is opened.

---

## 8. Wi-Fi (ESP32-S3)

### 8.1 Connection

- STA mode only (no AP mode in v1).
- Saved credentials in `/m5tui/config.yaml` under `wifi:`.
- Power-saving: `WIFI_PS_MIN_MODEM` after 30 s of no traffic.
- Auto-reconnect on disconnect.

### 8.2 RSSI sampling

`esp_wifi_sta_get_ap_info()` gives the current RSSI in dBm. We poll it
every 5 s and render `▁▃▆█` (4 bars) based on the threshold table.

| RSSI (dBm) | Bars |
|---:|:---:|
| ≥ -55 | ████ |
| -55 to -65 | ███▁ |
| -65 to -75 | ██▁▁ |
| -75 to -85 | █▁▁▁ |
| < -85 | ▁▁▁▁ |

---

## 9. Tailscale (userspace daemon)

### 9.1 Boot sequence

1. Wi-Fi connects.
2. `tailscale up --authkey=<key> --hostname=m5tui` is run.
3. Wait for `tailscale status` to show the device as `active`.
4. Resolve `aiserver-1` via MagicDNS.

### 9.2 Failure handling

If Tailscale fails to come up, m5Tui falls back to the LAN address in
the profile (`address_fallback: 192.168.4.91`). The top bar shows
`mesh: down` so the user knows.

### 9.3 Auth key

The Tailscale auth key is stored in `/m5tui/config.yaml` under
`tailscale.authkey`. It is `chmod 600` on write. The first-boot wizard
collects it from the user via a paste-in screen.

---

## 10. PMIC (AXP2101)

The AXP2101 manages:

- Battery charging.
- Backlight PWM.
- Multiple DCDC rails.

### 10.1 Battery %

Polled every 30 s via the AXP2101's `BATTERY_PERCENT` register. The
percentage is rendered in the top bar as `73%` (with a `⚡` glyph if
charging).

### 10.2 Low-battery behaviour

| Battery % | Action |
|---:|---|
| < 30 | Warn toast. |
| < 15 | Stronger warning. Suggest charging. |
| < 5 | Auto-save all pending writes. Show "please charge" banner. |
| < 2 | Graceful shutdown (saves state, syncs logs). |

### 10.3 Backlight

Driven by the AXP2101's DCDC3 (needs verification). The brightness
slider in the theme editor writes a value 0-100 to the AXP2101
register, with a small dead zone at the low end (0-5%) to avoid
backlight flicker.

---

## 11. Power budget

| State | Current draw (mA) | 1750 mAh life |
|---|---:|---:|
| Idle, screen off | ~80 | ~22 h |
| Idle, screen on 30% | ~140 | ~12 h |
| Active, screen on 60%, animations 1, sound on | ~280 | ~6 h |
| Active, screen on 60%, animations 2, sound on, mic recording | ~360 | ~5 h |
| Active, screen on 100%, animations 2, sound on, Wi-Fi TX | ~450 | ~4 h |

These are estimates; the actual numbers need to be measured on a real
device. The `;D` doctor should include a "battery life" estimate based
on the current draw and recent activity.

---

## 12. Pin summary (quick reference)

| Function | Pin | Direction | Notes |
|---|---|---|---|
| Display MOSI | G35 | OUT | SPI data |
| Display SCK | G36 | OUT | SPI clock |
| Display CS | G37 | OUT | SPI chip select |
| Display DC | G34 | OUT | data/command |
| Display RST | G33 | OUT | reset |
| Backlight | G38 | OUT | PWM (or via AXP2101) |
| Keyboard SDA | G8 | I/O | shared I2C |
| Keyboard SCL | G9 | OUT | shared I2C |
| Keyboard INT | G11 | IN | TCA8418 interrupt |
| Audio I2S BCLK | G41 | OUT | |
| Audio I2S DOUT | G46 | OUT | to codec DAC |
| Audio I2S DIN | G42 | IN | from codec ADC |
| Audio I2S LRCK | G43 | OUT | |
| IMU INT1 | G4 | IN | |
| IR TX | G44 | OUT | 38 kHz carrier |
| SD CS | G12 | OUT | |
| SD MOSI | G14 | OUT | shared with EXT |
| SD MISO | G39 | IN | shared with EXT |
| SD CLK | G40 | OUT | shared with EXT |
| PMIC IRQ | G3 | IN | |
| Boot mode | G0 | IN | hold low at power-on |

**Avoid:** G46 has strapping sensitivity on the Stamp-S3A. Don't drive
it during boot.

---

**End of HARDWARE.md.**
