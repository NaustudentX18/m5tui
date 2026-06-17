//! Host implementations of the device trait surface.
//!
//! These are used by `cargo test` and by simulator builds.  They deliberately
//! avoid real hardware dependencies so the crate compiles on any host target.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use m5tui_core::event::KeyAction;
use m5tui_core::framebuffer::Frame;
use m5tui_core::keymap::parse_chord;

use crate::traits::{
    AudioIn, AudioOut, DeviceError, Display, Imu, ImuSample, Keyboard, Sample, Storage,
};

/// In-memory display recording how many frames were pushed and the last
/// backlight level.
#[derive(Debug, Default, Clone)]
pub struct HostDisplay {
    push_count: u64,
    backlight_level: u8,
    initialized: bool,
}

impl HostDisplay {
    /// Number of frames pushed since init.
    pub fn push_count(&self) -> u64 {
        self.push_count
    }

    /// Last backlight level written, or 0 before init.
    pub fn backlight_level(&self) -> u8 {
        self.backlight_level
    }

    /// Whether `init()` has been called.
    pub fn initialized(&self) -> bool {
        self.initialized
    }
}

impl Display for HostDisplay {
    fn init(&mut self) -> Result<(), DeviceError> {
        self.initialized = true;
        self.push_count = 0;
        self.backlight_level = 0;
        Ok(())
    }

    fn push(&mut self, _frame: &Frame) -> Result<(), DeviceError> {
        if !self.initialized {
            return Err(DeviceError::Display("push before init".into()));
        }
        self.push_count += 1;
        Ok(())
    }

    fn backlight(&mut self, level: u8) -> Result<(), DeviceError> {
        if !self.initialized {
            return Err(DeviceError::Display("backlight before init".into()));
        }
        self.backlight_level = level;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), DeviceError> {
        self.initialized = false;
        Ok(())
    }
}

/// Keyboard that decodes a [`Read`] byte stream into [`KeyAction`]s.
///
/// Bytes are parsed as UTF-8 chars.  A leading `;` is remembered and combined
/// with the next char by `m5tui_core::keymap::parse_chord`; the first `;` is
/// returned as `KeyAction::Palette` so the host input layer can be used with
/// the same chord logic as the device keyboard driver.
#[derive(Debug)]
pub struct HostKeyboard<R: Read> {
    source: R,
    pending: Option<char>,
}

impl HostKeyboard<io::Stdin> {
    /// Keyboard backed by the process stdin.
    pub fn stdin() -> Self {
        Self::new(io::stdin())
    }
}

impl<R: Read> HostKeyboard<R> {
    /// Wrap any readable source.
    pub fn new(source: R) -> Self {
        Self {
            source,
            pending: None,
        }
    }

    /// Borrow the underlying source.
    pub fn source(&self) -> &R {
        &self.source
    }

    /// Mutably borrow the underlying source.
    pub fn source_mut(&mut self) -> &mut R {
        &mut self.source
    }
}

impl<R: Read + Send> Keyboard for HostKeyboard<R> {
    fn poll(&mut self) -> Result<Option<KeyAction>, DeviceError> {
        let mut byte = [0u8; 1];
        match self.source.read(&mut byte) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(e) => return Err(DeviceError::Keyboard(e.to_string())),
        }

        let c = byte[0] as char;
        // If there was a pending ';', combine it with the new byte first.
        if let Some(prev) = self.pending.take() {
            if prev == ';' {
                return Ok(parse_chord(Some(prev), c));
            }
        }

        if c == ';' {
            self.pending = Some(c);
            Ok(Some(KeyAction::Palette))
        } else {
            Ok(parse_chord(None, c))
        }
    }
}

/// Inert IMU that always returns a zero sample.
#[derive(Debug, Default, Clone, Copy)]
pub struct HostImu;

impl Imu for HostImu {
    fn sample(&mut self) -> Result<ImuSample, DeviceError> {
        Ok(ImuSample::default())
    }
}

/// Silence generator for audio input tests.
#[derive(Debug, Default, Clone)]
pub struct HostAudioIn {
    sample_rate: u32,
}

impl HostAudioIn {
    /// Create a silence generator at the given sample rate.
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

impl AudioIn for HostAudioIn {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, DeviceError> {
        for s in buf.iter_mut() {
            *s = 0;
        }
        Ok(buf.len())
    }
}

/// Audio sink that records every written sample.
#[derive(Debug, Default, Clone)]
pub struct HostAudioOut {
    sample_rate: u32,
    samples: Vec<Sample>,
    flushed: bool,
}

impl HostAudioOut {
    /// Create a recording sink at the given sample rate.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: Vec::new(),
            flushed: false,
        }
    }

    /// Borrow all recorded samples.
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    /// Whether `flush()` has been called at least once.
    pub fn flushed(&self) -> bool {
        self.flushed
    }
}

impl AudioOut for HostAudioOut {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn write(&mut self, samples: &[Sample]) -> Result<(), DeviceError> {
        self.samples.extend_from_slice(samples);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), DeviceError> {
        self.flushed = true;
        Ok(())
    }
}

/// Filesystem-backed storage.  All paths are resolved under `root`.
#[derive(Debug, Clone)]
pub struct HostStorage {
    root: PathBuf,
}

impl HostStorage {
    /// Create a storage driver rooted at `root`.  The directory is created if
    /// it does not exist.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, DeviceError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).map_err(|e| DeviceError::Storage(e.to_string()))?;
        Ok(Self { root })
    }

    /// Absolute path for a logical path.
    fn resolve(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    /// Borrow the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Storage for HostStorage {
    fn read(&self, path: &str) -> Result<Vec<u8>, DeviceError> {
        let resolved = self.resolve(path);
        std::fs::read(&resolved)
            .map_err(|e| DeviceError::Storage(format!("read {}: {e}", resolved.display())))
    }

    fn write(&self, path: &str, data: &[u8]) -> Result<(), DeviceError> {
        let resolved = self.resolve(path);
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DeviceError::Storage(e.to_string()))?;
        }
        let temp = resolved.with_extension("tmp");
        std::fs::write(&temp, data)
            .map_err(|e| DeviceError::Storage(format!("write temp: {e}")))?;
        std::fs::rename(&temp, &resolved).map_err(|e| {
            DeviceError::Storage(format!(
                "rename {} -> {}: {e}",
                temp.display(),
                resolved.display()
            ))
        })?;
        Ok(())
    }

    fn exists(&self, path: &str) -> Result<bool, DeviceError> {
        let resolved = self.resolve(path);
        Ok(resolved.exists())
    }

    fn remove(&self, path: &str) -> Result<(), DeviceError> {
        let resolved = self.resolve(path);
        let md = std::fs::metadata(&resolved)
            .map_err(|e| DeviceError::Storage(format!("metadata {}: {e}", resolved.display())))?;
        if md.is_dir() {
            std::fs::remove_dir_all(&resolved)
        } else {
            std::fs::remove_file(&resolved)
        }
        .map_err(|e| DeviceError::Storage(format!("remove {}: {e}", resolved.display())))
    }

    fn list(&self, path: &str) -> Result<Vec<String>, DeviceError> {
        let resolved = self.resolve(path);
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&resolved)
            .map_err(|e| DeviceError::Storage(format!("read_dir {}: {e}", resolved.display())))?
        {
            let entry = entry.map_err(|e| DeviceError::Storage(e.to_string()))?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }
}

/// In-memory storage driver.  Useful for unit tests that do not need real
/// filesystem state.
#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Storage for MemoryStorage {
    fn read(&self, path: &str) -> Result<Vec<u8>, DeviceError> {
        let files = self
            .files
            .lock()
            .map_err(|e| DeviceError::Storage(e.to_string()))?;
        files
            .get(path)
            .cloned()
            .ok_or_else(|| DeviceError::Storage(format!("not found: {path}")))
    }

    fn write(&self, path: &str, data: &[u8]) -> Result<(), DeviceError> {
        let mut files = self
            .files
            .lock()
            .map_err(|e| DeviceError::Storage(e.to_string()))?;
        files.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn exists(&self, path: &str) -> Result<bool, DeviceError> {
        let files = self
            .files
            .lock()
            .map_err(|e| DeviceError::Storage(e.to_string()))?;
        Ok(files.contains_key(path))
    }

    fn remove(&self, path: &str) -> Result<(), DeviceError> {
        let mut files = self
            .files
            .lock()
            .map_err(|e| DeviceError::Storage(e.to_string()))?;
        files
            .remove(path)
            .ok_or_else(|| DeviceError::Storage(format!("not found: {path}")))?;
        Ok(())
    }

    fn list(&self, _path: &str) -> Result<Vec<String>, DeviceError> {
        let files = self
            .files
            .lock()
            .map_err(|e| DeviceError::Storage(e.to_string()))?;
        let mut names: Vec<String> = files.keys().cloned().collect();
        names.sort();
        Ok(names)
    }
}

/// Convenience bundle of all host peripherals.  A real device driver will
/// provide an analogous `Device` struct that wires the hardware traits.
#[derive(Debug, Default, Clone)]
pub struct HostDevice {
    /// In-memory display recording.
    pub display: HostDisplay,
    /// Inert IMU.
    pub imu: HostImu,
    /// Recording audio sink.
    pub audio_out: HostAudioOut,
    /// Silence audio source.
    pub audio_in: HostAudioIn,
}

impl HostDevice {
    /// Create a host device with sensible defaults (16 kHz audio, zero sample).
    pub fn new() -> Self {
        Self {
            display: HostDisplay::default(),
            imu: HostImu,
            audio_out: HostAudioOut::new(16_000),
            audio_in: HostAudioIn::new(16_000),
        }
    }
}
