//! `m5tui-device` — ESP32-S3 / M5GFX device trait boundary for m5Tui.
//!
//! This crate defines the hardware traits ([`traits::Display`], [`traits::Keyboard`],
//! [`traits::Imu`], [`traits::AudioIn`], [`traits::AudioOut`], [`traits::Storage`])
//! that the rest of the workspace can depend on without knowing whether the
//! final binary is a host simulator or an ESP32-S3 firmware image.
//!
//! Real drivers will be implemented behind the `device` feature flag so that
//! host builds never pull in `esp-idf-*` dependencies.  The [`host`] module
//! provides fully-functional host implementations for tests and simulators.
//!
//! Host usage:
//! ```
//! use m5tui_device::host::{HostDevice, HostDisplay, HostStorage};
//! use m5tui_device::traits::{Display, Storage};
//!
//! let mut display = HostDisplay::default();
//! display.init().unwrap();
//! display.backlight(128).unwrap();
//!
//! let tmp = std::env::temp_dir().join("m5tui-device-example");
//! let storage = HostStorage::new(&tmp).unwrap();
//! storage.write("greeting.txt", b"hello device").unwrap();
//! ```

use m5tui_core::framebuffer::Frame;
use m5tui_core::layout::{FB_H, FB_W};
pub mod drivers;
pub mod host;
pub mod traits;

/// module path.
pub use traits::DeviceError;

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

/// Render a [`Frame`] to a 240x135 RGB565 buffer ready for `pushImage`.
pub fn frame_to_rgb565(frame: &Frame) -> Vec<u16> {
    rgba8888_to_rgb565(&m5tui_core::sim::render_to_rgba(frame))
}

#[cfg(test)]
mod tests {
    use super::*;
    use host::{
        HostAudioIn, HostAudioOut, HostDevice, HostDisplay, HostImu, HostKeyboard, HostStorage,
        MemoryStorage,
    };
    use m5tui_core::event::KeyAction;
    use m5tui_core::framebuffer::Frame;
    use std::io::Cursor;
    use traits::{AudioIn, AudioOut, Display, Imu, Keyboard, Storage};

    #[test]
    fn frame_to_rgb565_size_matches_display() {
        let frame = Frame::new_solid(0x0000);
        let buf = frame_to_rgb565(&frame);
        assert_eq!(buf.len(), FB_W * FB_H);
    }

    #[test]
    fn host_display_counts_pushes_and_backlight() {
        let mut d = HostDisplay::default();
        assert!(!d.initialized());
        d.init().unwrap();
        assert!(d.initialized());

        d.push(&Frame::new_solid(0x0000)).unwrap();
        d.push(&Frame::new_solid(0x1234)).unwrap();
        assert_eq!(d.push_count(), 2);

        d.backlight(77).unwrap();
        assert_eq!(d.backlight_level(), 77);

        d.shutdown().unwrap();
        assert!(!d.initialized());
    }

    #[test]
    fn host_display_rejects_push_before_init() {
        let mut d = HostDisplay::default();
        let result = d.push(&Frame::new_solid(0x0000));
        assert!(matches!(result, Err(DeviceError::Display(_))));
    }

    #[test]
    fn host_keyboard_decodes_chars_and_chords() {
        // HostKeyboard polls one byte at a time. Bare ';' is reported as
        // Palette and stashed internally; the next char is resolved as a
        // chord via m5tui_core::keymap::parse_chord.
        let input = ";?;/tx";
        let mut kb = HostKeyboard::new(Cursor::new(input.as_bytes()));

        assert_eq!(kb.poll().unwrap(), Some(KeyAction::Palette)); // lone ;
        assert_eq!(kb.poll().unwrap(), Some(KeyAction::Help)); // ;?
        assert_eq!(kb.poll().unwrap(), Some(KeyAction::Palette)); // lone ;
        assert_eq!(kb.poll().unwrap(), Some(KeyAction::Palette)); // ;/ chord
        assert_eq!(kb.poll().unwrap(), Some(KeyAction::Char('t')));
        assert_eq!(kb.poll().unwrap(), Some(KeyAction::Char('x')));
        assert_eq!(kb.poll().unwrap(), None); // EOF
    }

    #[test]
    fn host_imu_returns_zero_sample() {
        let mut imu = HostImu;
        let sample = imu.sample().unwrap();
        assert_eq!(sample.accel, [0, 0, 0]);
        assert_eq!(sample.gyro, [0, 0, 0]);
    }

    #[test]
    fn host_audio_round_trip() {
        let mut out = HostAudioOut::new(16_000);
        let mut input = HostAudioIn::new(16_000);

        assert_eq!(out.sample_rate(), 16_000);
        assert_eq!(input.sample_rate(), 16_000);

        let mut buf = [0i16; 64];
        let n = input.read(&mut buf).unwrap();
        assert_eq!(n, 64);
        assert!(buf.iter().all(|s| *s == 0));

        out.write(&buf).unwrap();
        assert_eq!(out.samples().len(), 64);
        assert!(!out.flushed());
        out.flush().unwrap();
        assert!(out.flushed());
    }

    #[test]
    fn host_storage_file_round_trip() {
        let tmp = std::env::temp_dir().join("m5tui-device-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let storage = HostStorage::new(&tmp).unwrap();

        storage.write("nested/data.bin", &[1, 2, 3]).unwrap();
        assert!(storage.exists("nested/data.bin").unwrap());

        let data = storage.read("nested/data.bin").unwrap();
        assert_eq!(data, vec![1, 2, 3]);

        let list = storage.list("").unwrap();
        assert!(list.contains(&"nested".to_string()));

        let nested = storage.list("nested").unwrap();
        assert_eq!(nested, vec!["data.bin"]);

        storage.remove("nested/data.bin").unwrap();
        assert!(!storage.exists("nested/data.bin").unwrap());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn memory_storage_works() {
        let storage = MemoryStorage::default();
        storage.write("foo/bar.txt", b"hello").unwrap();
        assert!(storage.exists("foo/bar.txt").unwrap());
        assert_eq!(storage.read("foo/bar.txt").unwrap(), b"hello");

        let names = storage.list("").unwrap();
        assert_eq!(names, vec!["foo/bar.txt"]);

        storage.remove("foo/bar.txt").unwrap();
        assert!(!storage.exists("foo/bar.txt").unwrap());
    }

    #[test]
    fn host_device_bundle_defaults() {
        let mut device = HostDevice::new();
        assert_eq!(device.audio_in.sample_rate(), 16_000);
        assert_eq!(device.audio_out.sample_rate(), 16_000);

        device.display.init().unwrap();
        device.imu.sample().unwrap();
    }
}

/// Device-only driver module.  This is empty scaffolding; the real ESP32
/// implementations of [`traits::Display`], [`traits::Keyboard`], etc. will be
/// added here behind the `device` feature in a later hardware phase.
#[cfg(feature = "device")]
pub mod device {
    //! Placeholder for `esp-idf-*` backed drivers.
    //!
    //! Real implementations will implement the traits in [`crate::traits`] by
    //! driving:
    //! - ST7789V2 SPI display via M5GFX or `esp-idf-hal` SPI,
    //! - TCA8418 I2C keyboard matrix,
    //! - BMI270 I2C IMU,
    //! - ES8311 I2S audio codec,
    //! - microSD SPI (CS G12) storage.

    use super::traits::DeviceError;

    /// Marker error returned if the device feature is enabled but no real
    /// driver has been wired yet.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DeviceNotImplemented;

    impl std::fmt::Display for DeviceNotImplemented {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "device driver not implemented yet")
        }
    }

    impl std::error::Error for DeviceNotImplemented {}

    /// Convert a [`DeviceNotImplemented`] marker into a [`DeviceError::Io`].
    pub fn not_implemented() -> DeviceError {
        DeviceError::Io("device driver not implemented yet".into())
    }
}
