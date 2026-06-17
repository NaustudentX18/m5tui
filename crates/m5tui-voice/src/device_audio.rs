//! Device audio implementation using esp-i2s / ES8311.
//!
//! This is intentionally stubbed for the host build. The real driver belongs
//! to the `xtensa-esp32s3-espidf` target and is gated by the `device`
//! feature.

use crate::{AudioIn, AudioOut, Sample, VoiceError};

/// Device-side microphone input (stub).
pub struct DeviceAudioIn {
    sample_rate: u32,
}

impl DeviceAudioIn {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

impl AudioIn for DeviceAudioIn {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read(&mut self, _buf: &mut [Sample]) -> Result<usize, VoiceError> {
        // TODO: wire I2S RX + ES8311 ADC on `xtensa-esp32s3-espidf`.
        Err(VoiceError::Io(
            "device audio input not implemented".to_string(),
        ))
    }
}

/// Device-side speaker output (stub).
pub struct DeviceAudioOut {
    sample_rate: u32,
}

impl DeviceAudioOut {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

impl AudioOut for DeviceAudioOut {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn write(&mut self, _samples: &[Sample]) -> Result<(), VoiceError> {
        // TODO: wire I2S TX + ES8311 DAC + NS4150B on `xtensa-esp32s3-espidf`.
        Err(VoiceError::Io(
            "device audio output not implemented".to_string(),
        ))
    }

    fn flush(&mut self) -> Result<(), VoiceError> {
        Ok(())
    }
}
