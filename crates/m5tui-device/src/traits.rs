//! Hardware trait boundary for the M5Stack Cardputer-Adv.
//!
//! These traits are intentionally small and blocking: real drivers will run
//! on a single-threaded ESP32-S3 main loop, while host builds use the
//! implementations in [`crate::host`].  Keeping the surface minimal lets later
//! hardware agents implement against it without dragging in async runtimes.

use m5tui_core::{event::KeyAction, framebuffer::Frame};

/// One mono PCM16 sample.  Voice WAVs and I2S buffers both use this format.
pub type Sample = i16;

/// Unified error type for every device peripheral.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    /// Display initialization or push failed.
    Display(String),
    /// Keyboard read failed.
    Keyboard(String),
    /// IMU sample failed.
    Imu(String),
    /// Audio start/read/write/flush failed.
    Audio(String),
    /// Storage read/write/list failed.
    Storage(String),
    /// Generic I/O error, usually a host `std::io::Error` mapped to string.
    Io(String),
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Display(s) => write!(f, "display: {s}"),
            Self::Keyboard(s) => write!(f, "keyboard: {s}"),
            Self::Imu(s) => write!(f, "imu: {s}"),
            Self::Audio(s) => write!(f, "audio: {s}"),
            Self::Storage(s) => write!(f, "storage: {s}"),
            Self::Io(s) => write!(f, "io: {s}"),
        }
    }
}

impl std::error::Error for DeviceError {}

/// Screen output.  The host implementation records pushes in memory; a real
/// driver will call M5GFX/ST7789V2 `pushImage` with the RGB565 buffer produced
/// by [`crate::frame_to_rgb565`].
pub trait Display: Send {
    /// Initialize the LCD, backlight, and rotation.
    fn init(&mut self) -> Result<(), DeviceError>;
    /// Push a [`Frame`] to the screen.
    fn push(&mut self, frame: &Frame) -> Result<(), DeviceError>;
    /// Set backlight duty cycle 0..=255.
    fn backlight(&mut self, level: u8) -> Result<(), DeviceError>;
    /// Power off the display.
    fn shutdown(&mut self) -> Result<(), DeviceError>;
}

/// Keyboard input.  Real drivers read the TCA8418 matrix; the host
/// implementation reads bytes from any [`std::io::Read`] source (stdin by
/// default) and converts them to [`KeyAction`] via `m5tui_core::keymap`.
pub trait Keyboard: Send {
    /// Poll for a single key event.  Returns `Ok(None)` when no key is ready.
    fn poll(&mut self) -> Result<Option<KeyAction>, DeviceError>;
}

/// One IMU sample.  Values are raw sensor readings; units are driver-defined
/// and documented here so callers can calibrate later.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImuSample {
    /// Accelerometer readings for X, Y, Z.
    pub accel: [i16; 3],
    /// Gyroscope readings for X, Y, Z.
    pub gyro: [i16; 3],
}

/// Inertial measurement unit.
pub trait Imu: Send {
    /// Read a single sample from the IMU.
    fn sample(&mut self) -> Result<ImuSample, DeviceError>;
}

/// Microphone input.
pub trait AudioIn: Send {
    /// Nominal sample rate, e.g. 16000 Hz.
    fn sample_rate(&self) -> u32;
    /// Fill `buf` with mono PCM16 samples.  Returns the number of samples read.
    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, DeviceError>;
}

/// Speaker output.
pub trait AudioOut: Send {
    /// Nominal sample rate, e.g. 16000 Hz.
    fn sample_rate(&self) -> u32;
    /// Write mono PCM16 samples to the output buffer.
    fn write(&mut self, samples: &[Sample]) -> Result<(), DeviceError>;
    /// Drain any buffered samples to the DAC.
    fn flush(&mut self) -> Result<(), DeviceError>;
}

/// Persistent storage.  Paths are driver-defined; the host driver treats them
/// as relative paths under a root directory, while a real SD driver maps them
/// to the microSD SPI filesystem.
pub trait Storage: Send {
    /// Read the entire contents of `path`.
    fn read(&self, path: &str) -> Result<Vec<u8>, DeviceError>;
    /// Atomically write `data` to `path`, creating parent directories if needed.
    fn write(&self, path: &str, data: &[u8]) -> Result<(), DeviceError>;
    /// Return true if `path` exists.
    fn exists(&self, path: &str) -> Result<bool, DeviceError>;
    /// Remove `path`.
    fn remove(&self, path: &str) -> Result<(), DeviceError>;
    /// List the entries in directory `path`.
    fn list(&self, path: &str) -> Result<Vec<String>, DeviceError>;
}
