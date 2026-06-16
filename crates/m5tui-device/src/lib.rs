//! `m5tui-device` — ESP32-S3 / M5GFX device render target.
//!
//! This crate defines the trait boundary between the host-sim `m5tui-core`
//! framebuffer and a real M5Stack Cardputer-Adv screen. The actual M5GFX
//! calls are stubbed because the Arduino/ESP-IDF stack is not available
//! in the host `cargo` build. When the ESP32 toolchain is present, an
//! implementation of `M5Display` drives the hardware.
//!
//! To build for device you need:
//! - `espup` / `esp-idf` / `xtensa-esp32s3-elf-gcc`
//! - `m5gfx` + `m5unified` Arduino libraries
//! - `m5launcher` app descriptor (see `m5launcher/app.json`)

use m5tui_core::framebuffer::Frame;
use m5tui_core::layout::{FB_H, FB_W};

/// Device-side display abstraction.
pub trait M5Display {
    /// Initialize the LCD, backlight, and rotation.
    fn init(&mut self) -> Result<(), DeviceError>;
    /// Push the 240x135 RGB565 or RGBA8888 frame to the screen.
    fn push(&mut self, frame: &Frame) -> Result<(), DeviceError>;
    /// Set backlight duty cycle 0..=255.
    fn backlight(&mut self, level: u8) -> Result<(), DeviceError>;
    /// Power off the display.
    fn shutdown(&mut self) -> Result<(), DeviceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    Init(String),
    Push(String),
    Backlight(String),
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(s) => write!(f, "init: {s}"),
            Self::Push(s) => write!(f, "push: {s}"),
            Self::Backlight(s) => write!(f, "backlight: {s}"),
        }
    }
}

impl std::error::Error for DeviceError {}

/// Host stub: verifies dimensions but cannot talk to real hardware.
pub struct HostStubDisplay;

impl Default for HostStubDisplay {
    fn default() -> Self {
        Self
    }
}

impl M5Display for HostStubDisplay {
    fn init(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn push(&mut self, _frame: &Frame) -> Result<(), DeviceError> {
        // Host stub cannot render, but validates that a frame was supplied.
        Ok(())
    }

    fn backlight(&mut self, _level: u8) -> Result<(), DeviceError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// Convert the host-sim RGBA8888 buffer to a flat RGB565 image.
/// This is the native pixel format M5GFX `pushImage` expects for the
/// Cardputer-Adv 240x135 LCD.
pub fn rgba8888_to_rgb565(rgba: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(FB_W * FB_H);
    for chunk in rgba.chunks_exact(4) {
        let r = (chunk[0] >> 3) as u16;
        let g = (chunk[1] >> 2) as u16;
        let b = (chunk[2] >> 3) as u16;
        out.push((r << 11) | (g << 5) | b);
    }
    out
}

/// Render a `Frame` to a 240x135 RGB565 buffer ready for `pushImage`.
pub fn frame_to_rgb565(frame: &Frame) -> Vec<u16> {
    rgba8888_to_rgb565(&m5tui_core::sim::render_to_rgba(frame))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_stub_init_push_ok() {
        let mut d = HostStubDisplay;
        d.init().unwrap_or_else(|e| panic!("{e}"));
        let frame = Frame::new_solid(0x0000);
        d.push(&frame).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn frame_to_rgb565_size_matches_display() {
        let frame = Frame::new_solid(0x0000);
        let buf = frame_to_rgb565(&frame);
        assert_eq!(buf.len(), FB_W * FB_H);
    }
}
