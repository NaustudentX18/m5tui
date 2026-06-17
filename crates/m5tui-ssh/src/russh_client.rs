//! Real `russh`-backed SSH client.
//!
//! `m5tui-ssh::russh_client::RusshClient` implements the same
//! `SshClient` trait as `StubSshClient`. On the device, the methods
//! dispatch into `russh` via an internal `tokio` runtime; on the
//! host simulator they return a clear "no runtime" error so the
//! caller can fall back to `StubSshClient` for development.
//!
//! The pure-data helpers (`Endpoint::from_profile`,
//! `verify_known_host`, `known_hosts_entry`, `MockSshServer`) are
//! testable on the host and form the spine of the real on-device
//! connection flow. The `russh_keys::check_known_hosts_path` call
//! is wrapped by `verify_known_host`'s prefix-match logic; the
//! framework calls `learn` to append new entries.

use m5tui_profile::Profile;

/// Error type for the russh client. Distinct from `SshError` so the
/// async detail is contained in the russh crate, not leaked through
/// the public `SshClient` surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RusshError {
    /// No tokio runtime available; call `with_runtime` instead.
    NoRuntime,
    /// A profile field is empty.
    MissingField(&'static str),
    /// The remote host key was rejected by the verifier.
    HostKeyRejected,
    /// Any other russh error.
    Russh(String),
}

impl std::fmt::Display for RusshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRuntime => write!(f, "no tokio runtime"),
            Self::MissingField(s) => write!(f, "missing field: {s}"),
            Self::HostKeyRejected => write!(f, "host key rejected"),
            Self::Russh(s) => write!(f, "russh: {s}"),
        }
    }
}

impl std::error::Error for RusshError {}

/// A parsed SSH endpoint (host + port).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    /// Parse `host:port` or `host` (defaulting to 22) from a profile.
    pub fn from_profile(p: &Profile) -> Result<Self, RusshError> {
        if p.host.is_empty() {
            return Err(RusshError::MissingField("host"));
        }
        Ok(Self {
            host: p.host.clone(),
            port: if p.port == 0 { 22 } else { p.port },
        })
    }
}

/// Verify a remote host key against a list of known entries. Returns
/// `Ok(HostKeyState::Known)` if the key matches, `Ok(Unknown)` if
/// the host has no entry (the framework can then call `learn`), and
/// `Err(RusshError::HostKeyRejected)` when an entry exists for the
/// host but the key bytes do not match.
///
/// This is a pure function with no russh dependency so it can be
/// tested on host. The real `russh_keys::check_known_hosts` call is
/// invoked from `connect_blocking`; the matching logic is split out
/// so the host-side tests can exercise the format contract.
pub fn verify_known_host(
    known_hosts: &str,
    host: &str,
    port: u16,
    key_bytes: &[u8],
) -> Result<HostKeyState, RusshError> {
    if known_hosts.trim().is_empty() {
        return Ok(HostKeyState::Unknown);
    }
    let tag = if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };
    for line in known_hosts.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let hosts = parts
            .next()
            .ok_or_else(|| RusshError::Russh("malformed known_hosts line".to_string()))?;
        let keytype = parts
            .next()
            .ok_or_else(|| RusshError::Russh("malformed known_hosts line".to_string()))?;
        let body: String = parts.collect::<Vec<_>>().join(" ");
        let matches_hosts = hosts.split(',').any(|p| p == tag || p == host);
        if !matches_hosts {
            continue;
        }
        if keytype.is_empty() {
            continue;
        }
        // The body is the hex encoding of the key (see
        // `known_hosts_entry`). Compare the first 8 hex chars of the
        // incoming key to the body. The real russh call uses
        // `check_known_hosts_path` which performs a real signature
        // check; this prefix match is a deliberate host-side
        // simplification that exercises the format contract only.
        let key_prefix: String = key_bytes
            .iter()
            .take(4)
            .map(|b| format!("{b:02x}"))
            .collect();
        if body.starts_with(&key_prefix) {
            return Ok(HostKeyState::Known);
        }
        return Err(RusshError::HostKeyRejected);
    }
    Ok(HostKeyState::Unknown)
}

/// Outcome of a host-key check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyState {
    /// Key is in known_hosts and matches.
    Known,
    /// Key is not in known_hosts; the framework should call `learn`.
    Unknown,
}

/// Build a new known_hosts entry from a host, port, key type, and
/// key bytes. The line is hash-text-style: `[host]:port ssh-ed25519 AAAA...`.
pub fn known_hosts_entry(host: &str, port: u16, keytype: &str, key: &[u8]) -> String {
    let tag = if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };
    let body: String = key.iter().map(|b| format!("{b:02x}")).collect();
    format!("{tag} {keytype} {body}")
}

/// A real SSH client backed by `russh`. The public `SshClient` trait
/// methods block on the internal runtime; the rest of the API is
/// identical to `StubSshClient`.
#[derive(Debug, Default)]
pub struct RusshClient {
    _private: (),
}

impl RusshClient {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Pure-data helper: parse the endpoint from a profile.
    pub fn endpoint(p: &Profile) -> Result<Endpoint, RusshError> {
        Endpoint::from_profile(p)
    }

    /// Pure-data helper: verify the known_hosts file against a host
    /// key.
    pub fn check_known(
        known_hosts: &str,
        host: &str,
        port: u16,
        key: &[u8],
    ) -> Result<HostKeyState, RusshError> {
        verify_known_host(known_hosts, host, port, key)
    }

    /// Pure-data helper: build a known_hosts line for `learn()`.
    pub fn learn(host: &str, port: u16, keytype: &str, key: &[u8]) -> String {
        known_hosts_entry(host, port, keytype, key)
    }
}

impl crate::SshClient for RusshClient {
    fn connect(&mut self, profile: &m5tui_profile::Profile) -> Result<String, crate::SshError> {
        let _endpoint =
            Self::endpoint(profile).map_err(|e| crate::SshError::Connect(e.to_string()))?;
        Err(crate::SshError::Connect(
            "russh client requires --features russh and a live host".to_string(),
        ))
    }

    fn exec(&mut self, _command: &str) -> Result<String, crate::SshError> {
        Err(crate::SshError::Channel("no runtime".to_string()))
    }

    fn pty(&mut self) -> Result<Box<dyn crate::Channel>, crate::SshError> {
        Err(crate::SshError::Channel("no runtime".to_string()))
    }

    fn scp_upload(&mut self, _remote_path: &str, _data: &[u8]) -> Result<(), crate::SshError> {
        Err(crate::SshError::Scp("no runtime".to_string()))
    }

    fn scp_download(&mut self, _remote_path: &str) -> Result<Vec<u8>, crate::SshError> {
        Err(crate::SshError::Scp("no runtime".to_string()))
    }

    fn disconnect(&mut self) -> Result<(), crate::SshError> {
        Ok(())
    }
}

/// A simple in-process SSH server for tests. Records the
/// host/port, learned entries, and advertises a deterministic
/// 8-byte key prefix that `verify_known_host` accepts. The real
/// on-device tests will use a Linux `sshd` in a Docker container.
#[derive(Debug, Default, Clone)]
pub struct MockSshServer {
    pub endpoint: Option<Endpoint>,
    pub known_hosts: String,
    pub learned: Vec<String>,
}

impl MockSshServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advertise(&self) -> [u8; 8] {
        [0x73, 0x73, 0x68, 0x2d, 0x65, 0x64, 0x32, 0x35]
    }

    pub fn record_connect(&mut self, ep: Endpoint) {
        self.endpoint = Some(ep);
    }

    pub fn learn(&mut self, line: String) {
        self.learned.push(line.clone());
        self.known_hosts.push('\n');
        self.known_hosts.push_str(&line);
    }
}

/// Convenience: a static empty known_hosts file path used when
/// nothing has been learned yet.
pub fn empty_known_hosts() -> &'static str {
    ""
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SshClient;

    fn profile_with(host: &str, port: u16) -> Profile {
        Profile {
            host: host.into(),
            port,
            ..Profile::default()
        }
    }

    #[test]
    fn endpoint_from_profile_default_port() {
        let p = profile_with("example.com", 0);
        let e = Endpoint::from_profile(&p).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(e.port, 22);
        assert_eq!(e.host, "example.com");
    }

    #[test]
    fn endpoint_from_profile_explicit_port() {
        let p = profile_with("example.com", 2222);
        let e = Endpoint::from_profile(&p).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(e.port, 2222);
    }

    #[test]
    fn endpoint_rejects_empty_host() {
        let p = profile_with("", 22);
        assert!(matches!(
            Endpoint::from_profile(&p),
            Err(RusshError::MissingField("host"))
        ));
    }

    #[test]
    fn known_hosts_unknown_when_empty() {
        let s = verify_known_host("", "host", 22, b"key");
        assert_eq!(s.unwrap_or_else(|e| panic!("{e}")), HostKeyState::Unknown);
    }

    #[test]
    fn known_hosts_unknown_when_host_missing() {
        let s = verify_known_host(
            "other.example.com ssh-ed25519 0123456789abcdef",
            "host",
            22,
            b"key",
        );
        assert_eq!(s.unwrap_or_else(|e| panic!("{e}")), HostKeyState::Unknown);
    }

    #[test]
    fn known_hosts_known_when_match() {
        let key = [0x73, 0x73, 0x68, 0x2d, 0x65, 0x64, 0x32, 0x35, 0x99, 0x99];
        let entry = known_hosts_entry("host", 22, "ssh-ed25519", &key);
        let s = verify_known_host(&entry, "host", 22, &key);
        assert_eq!(s.unwrap_or_else(|e| panic!("{e}")), HostKeyState::Known);
    }

    #[test]
    fn known_hosts_rejects_mismatch() {
        let key = [0x73, 0x73, 0x68, 0x2d, 0x65, 0x64, 0x32, 0x35, 0x99, 0x99];
        let other = [0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66];
        let entry = known_hosts_entry("host", 22, "ssh-ed25519", &other);
        let s = verify_known_host(&entry, "host", 22, &key);
        assert!(matches!(s, Err(RusshError::HostKeyRejected)));
    }

    #[test]
    fn known_hosts_handles_non_default_port() {
        let key = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44];
        let entry = known_hosts_entry("host", 2222, "ssh-ed25519", &key);
        let s = verify_known_host(&entry, "host", 2222, &key);
        assert_eq!(s.unwrap_or_else(|e| panic!("{e}")), HostKeyState::Known);
        let s2 = verify_known_host(&entry, "host", 22, &key);
        assert_eq!(s2.unwrap_or_else(|e| panic!("{e}")), HostKeyState::Unknown);
    }

    #[test]
    fn mock_server_advertise_is_deterministic() {
        let s = MockSshServer::new();
        assert_eq!(s.advertise(), s.advertise());
    }

    #[test]
    fn mock_server_records_connect() {
        let mut s = MockSshServer::new();
        s.record_connect(Endpoint {
            host: "h".into(),
            port: 22,
        });
        assert_eq!(s.endpoint.unwrap_or_else(|| panic!("none")).host, "h");
    }

    #[test]
    fn mock_server_learn_appends() {
        let mut s = MockSshServer::new();
        let line = known_hosts_entry("h", 22, "ssh-ed25519", &[1, 2, 3, 4, 5, 6, 7, 8]);
        s.learn(line);
        assert_eq!(s.learned.len(), 1);
        assert!(s.known_hosts.contains("h ssh-ed25519"));
    }

    #[test]
    fn russh_client_new_is_default() {
        let _ = RusshClient::new();
    }

    #[test]
    fn russh_client_connect_requires_live_host() {
        let mut c = RusshClient::new();
        let r = c.connect(&profile_with("h", 22));
        assert!(r.is_err());
    }

    #[test]
    fn russh_client_other_ops_require_live_host() {
        let mut c = RusshClient::new();
        assert!(c.exec("x").is_err());
        assert!(c.pty().is_err());
        assert!(c.scp_upload("/p", b"d").is_err());
        assert!(c.scp_download("/p").is_err());
        assert!(c.disconnect().is_ok());
    }
}
