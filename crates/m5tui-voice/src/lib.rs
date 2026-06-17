//! `m5tui-voice` — voice memo / PTT for m5Tui.
//!
//! M5 defines audio traits, a minimal WAV encoder/decoder, a PTT state
//! machine, and a directory-backed voice inbox. Real I2S/ES8311 drivers are
//! deferred to the hardware phase behind the `device` feature.

use std::path::{Path, PathBuf};

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

#[cfg(feature = "host-audio")]
mod host_audio;

#[cfg(feature = "host-audio")]
pub use host_audio::{HostAudioIn, HostAudioOut};

#[cfg(feature = "device")]
mod device_audio;

#[cfg(feature = "device")]
pub use device_audio::{DeviceAudioIn, DeviceAudioOut};

/// Push-to-talk state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttState {
    Idle,
    Recording { samples: usize, max: usize },
    Playing { samples: usize, max: usize },
    Menu,
}

impl PttState {
    pub fn new() -> Self {
        Self::Idle
    }

    pub fn hold_record(self, max_samples: usize) -> Result<Self, VoiceError> {
        match self {
            Self::Idle => Ok(Self::Recording {
                samples: 0,
                max: max_samples,
            }),
            other => Err(VoiceError::Ptt(format!("cannot record from {other:?}"))),
        }
    }

    pub fn release(self) -> Result<Self, VoiceError> {
        match self {
            Self::Recording { .. } => Ok(Self::Idle),
            other => Err(VoiceError::Ptt(format!("not recording: {other:?}"))),
        }
    }

    pub fn tap(self) -> Result<Self, VoiceError> {
        match self {
            Self::Idle => Ok(Self::Menu),
            Self::Menu => Ok(Self::Idle),
            other => Err(VoiceError::Ptt(format!("cannot tap from {other:?}"))),
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

impl VoiceMemo {
    pub fn from_wav(id: String, created: u64, wav: &[u8]) -> Result<Self, VoiceError> {
        let (samples, sample_rate) = Wav::decode_mono_pcm16(wav)?;
        Ok(Self {
            id,
            created,
            samples,
            sample_rate,
            pushed: false,
        })
    }

    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        (self.samples.len() as u64 * 1000) / self.sample_rate as u64
    }
}

/// Inbox abstraction.
pub trait VoiceInbox: Send + Sync {
    fn list(&self) -> Vec<&VoiceMemo>;
    fn get(&self, id: &str) -> Option<&VoiceMemo>;
    fn add(&mut self, memo: VoiceMemo) -> Result<(), VoiceError>;
    fn mark_pushed(&mut self, id: &str) -> Result<(), VoiceError>;
    fn remove(&mut self, id: &str) -> Result<(), VoiceError>;
    fn enqueue_plan(&mut self, id: &str, command: &str) -> Result<String, VoiceError>;
}

/// In-memory inbox.
#[derive(Debug, Default, Clone)]
pub struct InMemoryInbox {
    memos: Vec<VoiceMemo>,
    plan_queue: Vec<String>,
}

impl InMemoryInbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan_queue(&self) -> &[String] {
        &self.plan_queue
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

    fn enqueue_plan(&mut self, id: &str, command: &str) -> Result<String, VoiceError> {
        if self.get(id).is_none() {
            return Err(VoiceError::NotFound(id.to_string()));
        }
        let queued = format!("{command} --voice-memo {id}");
        self.plan_queue.push(queued.clone());
        Ok(queued)
    }
}

/// Directory-backed inbox that discovers `.wav` files on demand.
pub struct DirInbox {
    dir: PathBuf,
    memos: Vec<VoiceMemo>,
    plan_queue: Vec<String>,
}

impl DirInbox {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            memos: Vec::new(),
            plan_queue: Vec::new(),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn plan_queue(&self) -> &[String] {
        &self.plan_queue
    }

    pub fn refresh(&mut self) -> Result<(), VoiceError> {
        self.memos.clear();
        if !self.dir.exists() {
            return Ok(());
        }
        let mut entries: Vec<_> = std::fs::read_dir(&self.dir)
            .map_err(|e| VoiceError::Io(e.to_string()))?
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("wav"))
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let id = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let created = entry
                .metadata()
                .map_err(|e| VoiceError::Io(e.to_string()))?
                .modified()
                .map_err(|e| VoiceError::Io(e.to_string()))?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let data = std::fs::read(&path).map_err(|e| VoiceError::Io(e.to_string()))?;
            if let Ok(memo) = VoiceMemo::from_wav(id, created, &data) {
                self.memos.push(memo);
            }
        }
        Ok(())
    }
}

impl VoiceInbox for DirInbox {
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

    fn enqueue_plan(&mut self, id: &str, command: &str) -> Result<String, VoiceError> {
        if self.get(id).is_none() {
            return Err(VoiceError::NotFound(id.to_string()));
        }
        let queued = format!("{command} --voice-memo {id}");
        self.plan_queue.push(queued.clone());
        Ok(queued)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;

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
    fn memo_from_wav_round_trip() {
        let samples: Vec<Sample> = (0..80).map(|i| i as i16 * 50).collect();
        let wav = Wav::encode_mono_pcm16(&samples, 16000);
        let memo = VoiceMemo::from_wav("m1".to_string(), 1, &wav).unwrap();
        assert_eq!(memo.sample_rate, 16000);
        assert_eq!(memo.samples, samples);
        assert_eq!(memo.duration_ms(), 5);
    }

    #[test]
    fn ptt_idle_to_recording_to_idle() {
        let mut ptt = PttState::new();
        assert_eq!(ptt, PttState::Idle);
        ptt = ptt.hold_record(100).unwrap();
        assert!(matches!(ptt, PttState::Recording { .. }));
        ptt = ptt.release().unwrap();
        assert_eq!(ptt, PttState::Idle);
    }

    #[test]
    fn ptt_tap_opens_menu() {
        let mut ptt = PttState::new();
        ptt = ptt.tap().unwrap();
        assert_eq!(ptt, PttState::Menu);
        ptt = ptt.tap().unwrap();
        assert_eq!(ptt, PttState::Idle);
    }

    #[test]
    fn ptt_recording_stops_at_max() {
        let mut ptt = PttState::new()
            .hold_record(10)
            .unwrap_or_else(|e| panic!("{e}"));
        let done = ptt.record_samples(12).unwrap_or_else(|e| panic!("{e}"));
        assert!(done);
    }

    #[test]
    fn ptt_cannot_record_from_playing() {
        let ptt = PttState::new()
            .start_playing(10)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ptt.hold_record(10).is_err());
    }

    #[test]
    fn ptt_cannot_tap_while_recording() {
        let ptt = PttState::new().hold_record(10).unwrap();
        assert!(ptt.tap().is_err());
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

    #[test]
    fn inbox_enqueue_plan_command() {
        let mut inbox = InMemoryInbox::new();
        inbox
            .add(VoiceMemo {
                id: "m1".to_string(),
                created: 1,
                samples: vec![1, 2, 3],
                sample_rate: 16000,
                pushed: false,
            })
            .unwrap();
        let queued = inbox
            .enqueue_plan("m1", "advdeck-bridge plan --project garden")
            .unwrap();
        assert_eq!(
            queued,
            "advdeck-bridge plan --project garden --voice-memo m1"
        );
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{}_{}", prefix, std::process::id()))
    }

    #[test]
    fn dir_inbox_discovers_wav_files() {
        let tmp = temp_dir("m5tui_voice_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let wav = Wav::encode_mono_pcm16(&[42i16; 160], 16000);
        let path = tmp.join("memo.wav");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&wav).unwrap();
        }
        let mut inbox = DirInbox::new(&tmp);
        inbox.refresh().unwrap();
        let list = inbox.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "memo");
        assert_eq!(list[0].samples.len(), 160);
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn dir_inbox_enqueue_plan_uses_stem_id() {
        let tmp = temp_dir("m5tui_voice_plan_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let wav = Wav::encode_mono_pcm16(&[1i16; 16], 16000);
        let path = tmp.join("2026-06-15T22-04-11Z.wav");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&wav).unwrap();
        }
        let mut inbox = DirInbox::new(&tmp);
        inbox.refresh().unwrap();
        let queued = inbox
            .enqueue_plan(
                "2026-06-15T22-04-11Z",
                "advdeck-bridge plan --project garden",
            )
            .unwrap();
        assert_eq!(
            queued,
            "advdeck-bridge plan --project garden --voice-memo 2026-06-15T22-04-11Z"
        );
        assert_eq!(inbox.plan_queue().len(), 1);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
