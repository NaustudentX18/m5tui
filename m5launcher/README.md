# m5launcher packaging

This directory contains the M5Launcher app descriptor for m5Tui.

**Current status:** Template only. The real `.bin` file cannot be built
on the Pi because it requires the ESP32-S3 toolchain and M5GFX. Once
you have a firmware binary, place it here:

```
SD://m5tui/
  m5tui.bin   <- device firmware
  icon.png    <- 64x64 app icon
```

And copy `m5launcher/app.json` to:

```
SD://apps/m5Tui.json
```

(or the equivalent location your M5Launcher version expects).
