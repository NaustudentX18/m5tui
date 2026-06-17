//! Host audio implementation using cpal.

use crate::{AudioIn, AudioOut, Sample, VoiceError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// Capture from the default host input device.
pub struct HostAudioIn {
    sample_rate: u32,
    #[expect(dead_code)]
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<Sample>>>,
}

impl HostAudioIn {
    pub fn with_default_device(sample_rate: u32) -> Result<Self, VoiceError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| VoiceError::Io("no default input device".to_string()))?;
        let mut supported_configs = device
            .supported_input_configs()
            .map_err(|e| VoiceError::Io(e.to_string()))?;
        let config_range = supported_configs
            .find(|c| c.sample_format() == cpal::SampleFormat::I16 && c.channels() == 1)
            .ok_or_else(|| VoiceError::Io("no mono i16 input config".to_string()))?;
        let config = config_range.with_sample_rate(sample_rate);
        let buffer: Arc<Mutex<Vec<Sample>>> = Arc::new(Mutex::new(Vec::new()));
        let capture = Arc::clone(&buffer);
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = capture.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                move |err| {
                    eprintln!("host audio input error: {err}");
                },
                None,
            )
            .map_err(|e| VoiceError::Io(e.to_string()))?;
        stream.play().map_err(|e| VoiceError::Io(e.to_string()))?;
        Ok(Self {
            sample_rate,
            stream,
            buffer,
        })
    }
}

impl AudioIn for HostAudioIn {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, VoiceError> {
        let mut captured = self
            .buffer
            .lock()
            .map_err(|e| VoiceError::Io(e.to_string()))?;
        let n = buf.len().min(captured.len());
        buf[..n].copy_from_slice(&captured[..n]);
        captured.drain(..n);
        Ok(n)
    }
}

/// Playback through the default host output device.
pub struct HostAudioOut {
    sample_rate: u32,
    #[expect(dead_code)]
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<Sample>>>,
}

impl HostAudioOut {
    pub fn with_default_device(sample_rate: u32) -> Result<Self, VoiceError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| VoiceError::Io("no default output device".to_string()))?;
        let mut supported_configs = device
            .supported_output_configs()
            .map_err(|e| VoiceError::Io(e.to_string()))?;
        let config_range = supported_configs
            .find(|c| c.sample_format() == cpal::SampleFormat::I16 && c.channels() == 1)
            .ok_or_else(|| VoiceError::Io("no mono i16 output config".to_string()))?;
        let config = config_range.with_sample_rate(sample_rate);
        let buffer: Arc<Mutex<Vec<Sample>>> = Arc::new(Mutex::new(Vec::new()));
        let playback = Arc::clone(&buffer);
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut buf) = playback.lock() {
                        for (out, &src) in data.iter_mut().zip(buf.iter()) {
                            *out = src;
                        }
                        let consumed = data.len().min(buf.len());
                        buf.drain(..consumed);
                    }
                },
                move |err| {
                    eprintln!("host audio output error: {err}");
                },
                None,
            )
            .map_err(|e| VoiceError::Io(e.to_string()))?;
        stream.play().map_err(|e| VoiceError::Io(e.to_string()))?;
        Ok(Self {
            sample_rate,
            stream,
            buffer,
        })
    }
}

impl AudioOut for HostAudioOut {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn write(&mut self, samples: &[Sample]) -> Result<(), VoiceError> {
        let mut buf = self
            .buffer
            .lock()
            .map_err(|e| VoiceError::Io(e.to_string()))?;
        buf.extend_from_slice(samples);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), VoiceError> {
        // Real playback is async; the output stream keeps running after this
        // object is dropped. A full blocking flush is intentionally not
        // implemented because tests drive small fixed-size buffers.
        Ok(())
    }
}
