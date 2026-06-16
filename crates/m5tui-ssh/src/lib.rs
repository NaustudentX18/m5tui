//! `m5tui-ssh` — SSH client abstraction for m5Tui.
//!
//! M3 only defines traits and a `StubSshClient` that returns canned
//! responses. A real `russh`-based client will live behind the same
//! traits in a future `m5tui-ssh-russh` crate or device-specific impl.

use std::collections::HashMap;

/// A PTY size request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

/// A single channel to a remote shell or command.
pub trait Channel: Send + Sync {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SshError>;
    fn write(&mut self, data: &[u8]) -> Result<(), SshError>;
    fn resize(&mut self, size: PtySize) -> Result<(), SshError>;
    fn close(self) -> Result<(), SshError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshError {
    Connect(String),
    Auth(String),
    Channel(String),
    Scp(String),
    NotConnected,
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(msg) => write!(f, "connect: {msg}"),
            Self::Auth(msg) => write!(f, "auth: {msg}"),
            Self::Channel(msg) => write!(f, "channel: {msg}"),
            Self::Scp(msg) => write!(f, "scp: {msg}"),
            Self::NotConnected => write!(f, "not connected"),
        }
    }
}

impl std::error::Error for SshError {}

/// SSH client operations needed by m5Tui.
pub trait SshClient: Send + Sync {
    fn connect(&mut self, profile: &m5tui_profile::Profile) -> Result<String, SshError>;
    fn exec(&mut self, command: &str) -> Result<String, SshError>;
    fn pty(&mut self) -> Result<Box<dyn Channel>, SshError>;
    fn scp_upload(&mut self, remote_path: &str, data: &[u8]) -> Result<(), SshError>;
    fn scp_download(&mut self, remote_path: &str) -> Result<Vec<u8>, SshError>;
    fn disconnect(&mut self) -> Result<(), SshError>;
}

/// A canned channel that echoes whatever was written or returns preset
/// bytes on reads.
pub struct StubChannel {
    read_buf: Vec<u8>,
    written: Vec<u8>,
    closed: bool,
}

impl StubChannel {
    pub fn new(read_buf: &[u8]) -> Self {
        Self {
            read_buf: read_buf.to_vec(),
            written: Vec::new(),
            closed: false,
        }
    }

    pub fn written(&self) -> &[u8] {
        &self.written
    }
}

impl Channel for StubChannel {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SshError> {
        if self.closed {
            return Err(SshError::Channel("closed".to_string()));
        }
        let n = buf.len().min(self.read_buf.len());
        buf[..n].copy_from_slice(&self.read_buf[..n]);
        self.read_buf.drain(..n);
        Ok(n)
    }

    fn write(&mut self, data: &[u8]) -> Result<(), SshError> {
        if self.closed {
            return Err(SshError::Channel("closed".to_string()));
        }
        self.written.extend_from_slice(data);
        Ok(())
    }

    fn resize(&mut self, _size: PtySize) -> Result<(), SshError> {
        Ok(())
    }

    fn close(mut self) -> Result<(), SshError> {
        self.closed = true;
        Ok(())
    }
}

/// A stub client that maps known hosts to canned outputs.
pub struct StubSshClient {
    connected: Option<String>,
    file_system: HashMap<String, Vec<u8>>,
}

impl Default for StubSshClient {
    fn default() -> Self {
        let mut fs = HashMap::new();
        fs.insert(
            "/etc/hostname".to_string(),
            b"aiserver-1
"
            .to_vec(),
        );
        Self {
            connected: None,
            file_system: fs,
        }
    }
}

impl StubSshClient {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SshClient for StubSshClient {
    fn connect(&mut self, profile: &m5tui_profile::Profile) -> Result<String, SshError> {
        let id = format!("session-{}", profile.id);
        self.connected = Some(profile.id.clone());
        Ok(id)
    }

    fn exec(&mut self, command: &str) -> Result<String, SshError> {
        if self.connected.is_none() {
            return Err(SshError::NotConnected);
        }
        if command == "hostname" {
            Ok("aiserver-1
"
            .to_string())
        } else {
            Ok(format!(
                "ok: {command}
"
            ))
        }
    }

    fn pty(&mut self) -> Result<Box<dyn Channel>, SshError> {
        if self.connected.is_none() {
            return Err(SshError::NotConnected);
        }
        Ok(Box::new(StubChannel::new(b"aiserver-1 $ ")))
    }

    fn scp_upload(&mut self, remote_path: &str, data: &[u8]) -> Result<(), SshError> {
        if self.connected.is_none() {
            return Err(SshError::NotConnected);
        }
        self.file_system
            .insert(remote_path.to_string(), data.to_vec());
        Ok(())
    }

    fn scp_download(&mut self, remote_path: &str) -> Result<Vec<u8>, SshError> {
        if self.connected.is_none() {
            return Err(SshError::NotConnected);
        }
        self.file_system
            .get(remote_path)
            .cloned()
            .ok_or_else(|| SshError::Scp(format!("not found: {remote_path}")))
    }

    fn disconnect(&mut self) -> Result<(), SshError> {
        self.connected = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use m5tui_profile::{InMemoryRegistry, Profile, ProfileRegistry};

    fn profile() -> Profile {
        InMemoryRegistry::with_sample()
            .default()
            .unwrap_or_else(|| panic!("no default"))
            .clone()
    }

    #[test]
    fn stub_connect_returns_session_id() {
        let mut c = StubSshClient::new();
        let sid = c.connect(&profile()).unwrap_or_else(|e| panic!("{e}"));
        assert!(sid.starts_with("session-"));
    }

    #[test]
    fn stub_exec_hostname() {
        let mut c = StubSshClient::new();
        c.connect(&profile()).unwrap_or_else(|e| panic!("{e}"));
        let out = c.exec("hostname").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            out,
            "aiserver-1
"
        );
    }

    #[test]
    fn stub_exec_requires_connection() {
        let mut c = StubSshClient::new();
        assert!(c.exec("hostname").is_err());
    }

    #[test]
    fn stub_scp_round_trip() {
        let mut c = StubSshClient::new();
        c.connect(&profile()).unwrap_or_else(|e| panic!("{e}"));
        c.scp_upload("/tmp/memo.wav", b"wav")
            .unwrap_or_else(|e| panic!("{e}"));
        let got = c
            .scp_download("/tmp/memo.wav")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, b"wav");
    }

    #[test]
    fn stub_pty_channel_writes_and_reads() {
        let mut c = StubSshClient::new();
        c.connect(&profile()).unwrap_or_else(|e| panic!("{e}"));
        let mut ch = c.pty().unwrap_or_else(|e| panic!("{e}"));
        ch.write(
            b"ls
",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut buf = [0u8; 64];
        let n = ch.read(&mut buf).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(&buf[..n], b"aiserver-1 $ ");
    }
}
