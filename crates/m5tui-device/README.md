# m5tui-device

ESP32-S3 / M5GFX device render target for m5Tui.

This crate is a host-compilable stub. It defines `M5Display`, helpers to
convert the `m5tui-core` framebuffer to RGB565/RGBA8888, and a
`HostStubDisplay` for unit tests. A real device build must provide an
`M5Display` implementation backed by M5GFX on the Cardputer-Adv.

Build this crate on host:
```bash
cargo build -p m5tui-device
```

Build for device (not yet configured; requires espup + M5GFX):
```bash
cargo build -p m5tui-device --target xtensa-esp32s3-espidf
```
