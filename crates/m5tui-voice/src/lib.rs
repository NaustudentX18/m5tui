//! `m5tui-voice` — voice memo / PTT stubs for m5Tui.
//!
//! M5 defines audio traits, a minimal WAV encoder/decoder, a PTT state
//! machine, and an in-memory inbox. Real I2S/ES8311 drivers are deferred
//! to the hardware phase.

/// One mono PCM16 sample.
pub type Sample = i16;

/// Error type for voice operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceError {
    Io(String),
    Format(String),
    Ptt(String),
    NotFound(String),
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) => write!(f, "io: {s}"),
            Self::Format(s) => write!(f, "format: {s}"),
            Self::Ptt(s) => write!(f, "ptt: {s}"),
            Self::NotFound(s) => write!(f, "not found: {s}"),
        }
    }
}

impl std::error::Error for VoiceError {}

/// Microphone input trait.
pub trait AudioIn: Send + Sync {
    fn sample_rate(&self) -> u32;
    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, VoiceError>;
}

/// Speaker output trait.
pub trait AudioOut: Send + Sync {
    fn sample_rate(&self) -> u32;
    fn write(&mut self, samples: &[Sample]) -> Result<(), VoiceError>;
    fn flush(&mut self) -> Result<(), VoiceError>;
}

/// Silence generator for testing.
#[derive(Debug, Default, Clone)]
pub struct NullIn {
    sample_rate: u32,
}

impl NullIn {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

impl AudioIn for NullIn {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, VoiceError> {
        for s in buf.iter_mut() {
            *s = 0;
        }
        Ok(buf.len())
    }
}

/// Sink that counts samples and records them.
#[derive(Debug, Default, Clone)]
pub struct MemoryOut {
    sample_rate: u32,
    pub samples: Vec<Sample>,
}

impl MemoryOut {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: Vec::new(),
        }
    }
}

impl AudioOut for MemoryOut {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn write(&mut self, samples: &[Sample]) -> Result<(), VoiceError> {
        self.samples.extend_from_slice(samples);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), VoiceError> {
        Ok(())
    }
}

/// Records from AudioIn into a Vec.
#[derive(Debug, Default, Clone)]
pub struct MemoryIn {
    sample_rate: u32,
    pub samples: Vec<Sample>,
    position: usize,
}

impl MemoryIn {
    pub fn new(samples: Vec<Sample>, sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples,
            position: 0,
        }
    }
}

impl AudioIn for MemoryIn {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read(&mut self, buf: &mut [Sample]) -> Result<usize, VoiceError> {
        let remaining = self.samples.len().saturating_sub(self.position);
        let n = buf.len().min(remaining);
        if n == 0 {
            return Ok(0);
        }
        buf[..n].copy_from_slice(&self.samples[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}

/// Push-to-talk state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttState {
    Idle,
    Recording { samples: usize, max: usize },
    Playing { samples: usize, max: usize },
}

impl PttState {
    pub fn new() -> Self {
        Self::Idle
    }

    pub fn start_recording(self, max_samples: usize) -> Result<Self, VoiceError> {
        match self {
            Self::Idle => Ok(Self::Recording {
                samples: 0,
                max: max_samples,
            }),
            other => Err(VoiceError::Ptt(format!("cannot record from {other:?}"))),
        }
    }

    pub fn stop_recording(self) -> Result<(Self, usize), VoiceError> {
        match self {
            Self::Recording { samples, .. } => Ok((Self::Idle, samples)),
            other => Err(VoiceError::Ptt(format!("not recording: {other:?}"))),
        }
    }

    pub fn start_playing(self, max_samples: usize) -> Result<Self, VoiceError> {
        match self {
            Self::Idle => Ok(Self::Playing {
                samples: 0,
                max: max_samples,
            }),
            other => Err(VoiceError::Ptt(format!("cannot play from {other:?}"))),
        }
    }

    pub fn stop_playing(self) -> Result<Self, VoiceError> {
        match self {
            Self::Playing { .. } => Ok(Self::Idle),
            other => Err(VoiceError::Ptt(format!("not playing: {other:?}"))),
        }
    }

    pub fn record_samples(&mut self, n: usize) -> Result<bool, VoiceError> {
        match self {
            Self::Recording { samples, max } => {
                *samples = (*samples + n).min(*max);
                Ok(*samples >= *max)
            }
            other => Err(VoiceError::Ptt(format!("not recording: {other:?}"))),
        }
    }

    pub fn play_samples(&mut self, n: usize) -> Result<bool, VoiceError> {
        match self {
            Self::Playing { samples, max } => {
                *samples = (*samples + n).min(*max);
                Ok(*samples >= *max)
            }
            other => Err(VoiceError::Ptt(format!("not playing: {other:?}"))),
        }
    }
}

impl Default for PttState {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal mono PCM16 WAV encoder/decoder.
pub struct Wav;

impl Wav {
    pub fn encode_mono_pcm16(samples: &[Sample], sample_rate: u32) -> Vec<u8> {
        let data_size = samples.len() * 2;
        let total_size = 44 + data_size;
        let mut out = Vec::with_capacity(total_size);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((total_size - 8) as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes()); // subchunk size
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        out.extend_from_slice(&2u16.to_le_bytes()); // block align
        out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data_size as u32).to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    pub fn decode_mono_pcm16(data: &[u8]) -> Result<(Vec<Sample>, u32), VoiceError> {
        if data.len() < 44 {
            return Err(VoiceError::Format("file too small".to_string()));
        }
        if &data[..4] != b"RIFF" {
            return Err(VoiceError::Format("missing RIFF".to_string()));
        }
        if &data[8..12] != b"WAVE" {
            return Err(VoiceError::Format("missing WAVE".to_string()));
        }
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let data_size = u32::from_le_bytes([data[40], data[41], data[42], data[43]]) as usize;
        let sample_count = data_size / 2;
        let mut samples = Vec::with_capacity(sample_count);
        let data_start = 44;
        if data.len() < data_start + data_size {
            return Err(VoiceError::Format("truncated data".to_string()));
        }
        for i in 0..sample_count {
            let offset = data_start + i * 2;
            let s = i16::from_le_bytes([data[offset], data[offset + 1]]);
            samples.push(s);
        }
        Ok((samples, sample_rate))
    }
}

/// A saved voice memo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceMemo {
    pub id: String,
    pub created: u64,
    pub samples: Vec<Sample>,
    pub sample_rate: u32,
    pub pushed: bool,
}

/// Inbox abstraction.
pub trait VoiceInbox: Send + Sync {
    fn list(&self) -> Vec<&VoiceMemo>;
    fn get(&self, id: &str) -> Option<&VoiceMemo>;
    fn add(&mut self, memo: VoiceMemo) -> Result<(), VoiceError>;
    fn mark_pushed(&mut self, id: &str) -> Result<(), VoiceError>;
    fn remove(&mut self, id: &str) -> Result<(), VoiceError>;
}

/// In-memory inbox.
#[derive(Debug, Default, Clone)]
pub struct InMemoryInbox {
    memos: Vec<VoiceMemo>,
}

impl InMemoryInbox {
    pub fn new() -> Self {
        Self::default()
    }
}

impl VoiceInbox for InMemoryInbox {
    fn list(&self) -> Vec<&VoiceMemo> {
        self.memos.iter().collect()
    }

    fn get(&self, id: &str) -> Option<&VoiceMemo> {
        self.memos.iter().find(|m| m.id == id)
    }

    fn add(&mut self, memo: VoiceMemo) -> Result<(), VoiceError> {
        self.memos.push(memo);
        Ok(())
    }

    fn mark_pushed(&mut self, id: &str) -> Result<(), VoiceError> {
        let memo = self
            .memos
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| VoiceError::NotFound(id.to_string()))?;
        memo.pushed = true;
        Ok(())
    }

    fn remove(&mut self, id: &str) -> Result<(), VoiceError> {
        let idx = self
            .memos
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| VoiceError::NotFound(id.to_string()))?;
        self.memos.remove(idx);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_round_trip() {
        let samples: Vec<Sample> = (0..100).map(|i| i as i16 * 100 - 5000).collect();
        let wav = Wav::encode_mono_pcm16(&samples, 16000);
        let (decoded, rate) = Wav::decode_mono_pcm16(&wav).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(rate, 16000);
        assert_eq!(decoded, samples);
    }

    #[test]
    fn wav_rejects_truncated() {
        let mut wav = Wav::encode_mono_pcm16(&[0i16; 4], 16000);
        wav.truncate(20);
        assert!(Wav::decode_mono_pcm16(&wav).is_err());
    }

    #[test]
    fn ptt_recording_stops_at_max() {
        let mut ptt = PttState::new()
            .start_recording(10)
            .unwrap_or_else(|e| panic!("{e}"));
        let done = ptt.record_samples(12).unwrap_or_else(|e| panic!("{e}"));
        assert!(done);
    }

    #[test]
    fn ptt_cannot_record_from_playing() {
        let ptt = PttState::new()
            .start_playing(10)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ptt.start_recording(10).is_err());
    }

    #[test]
    fn inbox_add_and_mark_pushed() {
        let mut inbox = InMemoryInbox::new();
        inbox
            .add(VoiceMemo {
                id: "m1".to_string(),
                created: 1,
                samples: vec![1, 2, 3],
                sample_rate: 16000,
                pushed: false,
            })
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(!inbox.get("m1").unwrap_or_else(|| panic!("missing")).pushed);
        inbox.mark_pushed("m1").unwrap_or_else(|e| panic!("{e}"));
        assert!(inbox.get("m1").unwrap_or_else(|| panic!("missing")).pushed);
    }
}
