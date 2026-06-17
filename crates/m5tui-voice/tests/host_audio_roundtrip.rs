//! Host-only integration test: record 1 second of audio through cpal,
//! encode it to WAV, decode it back, and assert sample count round-trips.
//!
//! The test is gated by the `host-audio` feature. If cpal cannot open the
//! default input device, the test is skipped at runtime with a clear
//! message.

#![cfg(feature = "host-audio")]
#![allow(clippy::expect_used)]

use m5tui_voice::{AudioIn, AudioOut, HostAudioIn, HostAudioOut, Sample, Wav};
use std::thread;
use std::time::Duration;

const SAMPLE_RATE: u32 = 16_000;
const SECONDS: u32 = 1;
const EXPECTED_SAMPLES: usize = (SAMPLE_RATE * SECONDS) as usize;

#[test]
fn host_audio_record_one_second_roundtrip() {
    let mut input = match HostAudioIn::with_default_device(SAMPLE_RATE) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("SKIPPING host audio roundtrip: cpal cannot open default input device: {e}");
            return;
        }
    };

    let mut captured = Vec::with_capacity(EXPECTED_SAMPLES);
    let chunk_size = 1_024usize;
    let mut buf = vec![0i16; chunk_size];

    // Capture until we have roughly one second of samples.
    while captured.len() < EXPECTED_SAMPLES {
        match input.read(&mut buf) {
            Ok(0) => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(n) => captured.extend_from_slice(&buf[..n]),
            Err(e) => panic!("read error: {e}"),
        }
    }
    captured.truncate(EXPECTED_SAMPLES);

    // Encode / decode round-trip.
    let wav = Wav::encode_mono_pcm16(&captured, SAMPLE_RATE);
    let (decoded, rate) = Wav::decode_mono_pcm16(&wav).expect("decode own WAV");
    assert_eq!(rate, SAMPLE_RATE);
    assert_eq!(
        decoded.len(),
        EXPECTED_SAMPLES,
        "WAV round-trip preserved sample count"
    );

    // Verify output can be opened (do not block on real playback).
    let mut output = match HostAudioOut::with_default_device(SAMPLE_RATE) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("SKIPPING output check: cpal cannot open default output device: {e}");
            return;
        }
    };
    output
        .write(&decoded[..decoded.len().min(chunk_size)])
        .expect("write to host output");
}

#[test]
fn host_audio_sine_wave_roundtrip() {
    // Generate a 1 kHz mono sine wave; this test does not require hardware.
    let samples: Vec<Sample> = (0..EXPECTED_SAMPLES)
        .map(|i| {
            let t = i as f64 / SAMPLE_RATE as f64;
            let phase = 2.0 * std::f64::consts::PI * 1_000.0 * t;
            (phase.sin() * i16::MAX as f64) as i16
        })
        .collect();

    let wav = Wav::encode_mono_pcm16(&samples, SAMPLE_RATE);
    let (decoded, rate) = Wav::decode_mono_pcm16(&wav).expect("decode sine WAV");
    assert_eq!(rate, SAMPLE_RATE);
    assert_eq!(decoded.len(), EXPECTED_SAMPLES);
    assert_eq!(decoded, samples);

    // Optionally feed the sine wave to the default output if hardware exists.
    if let Ok(mut output) = HostAudioOut::with_default_device(SAMPLE_RATE) {
        let chunk = &decoded[..decoded.len().min(1_024)];
        output.write(chunk).expect("write sine to host output");
    }
}
