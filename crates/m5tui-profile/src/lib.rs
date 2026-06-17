//! `m5tui-profile` — connection profile registry.
//!
//! A [`Profile`] bundles everything needed to reach a remote host: SSH
//! credentials, an optional jump host, and an OMP profile name. The crate
//! provides:
//!
//! - YAML loader using `serde` + `serde_yaml`.
//! - [`FileRegistry`] that watches a directory, hot-reloads on mtime change,
//!   and tracks a default profile.
//! - [`InMemoryRegistry`] for tests and sim builds.
//! - [`resolve_proxy_jump`] to expand a profile into a chain of hosts (SSH
//!   `ProxyJump` semantics).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use m5tui_persist::Driver;
use serde::{Deserialize, Serialize};
/// A connection profile.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub host: String,
    pub user: String,
    /// Path to an SSH private key or empty for agent-based auth.
    #[serde(default)]
    pub key_path: String,
    /// Optional jump-host profile id.
    #[serde(default, alias = "jump")]
    pub jump_profile_id: Option<String>,
    /// OMP profile name used by the orchestrator sidecar.
    #[serde(default)]
    pub omp_profile: String,
    /// Whether this profile is the default when the app starts.
    #[serde(default)]
    pub default: bool,
}
fn default_port() -> u16 {
    22
}

impl Profile {
    /// Validation used before a profile is saved.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.id.is_empty() {
            return Err(ProfileError::Invalid("id is empty".to_string()));
        }
        if self.host.is_empty() {
            return Err(ProfileError::Invalid("host is empty".to_string()));
        }
        if self.user.is_empty() {
            return Err(ProfileError::Invalid("user is empty".to_string()));
        }
        if self.port == 0 {
            return Err(ProfileError::Invalid("port is 0".to_string()));
        }
        Ok(())
    }

    /// Build a display string for the profile picker.
    pub fn display_name(&self) -> String {
        format!("{}@{} — {}", self.user, self.host, self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    Invalid(String),
    NotFound(String),
    Duplicate(String),
    Persist(String),
    Yaml { line: usize, reason: String },
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(msg) => write!(f, "invalid profile: {msg}"),
            Self::NotFound(id) => write!(f, "profile not found: {id}"),
            Self::Duplicate(id) => write!(f, "duplicate profile: {id}"),
            Self::Persist(msg) => write!(f, "persist error: {msg}"),
            Self::Yaml { line, reason } => write!(f, "yaml error at line {line}: {reason}"),
        }
    }
}

impl std::error::Error for ProfileError {}

impl From<m5tui_persist::PersistError> for ProfileError {
    fn from(e: m5tui_persist::PersistError) -> Self {
        Self::Persist(e.to_string())
    }
}

/// Storage abstraction for profiles.
pub trait ProfileRegistry: Send + Sync {
    fn list(&self) -> Vec<&Profile>;
    fn get(&self, id: &str) -> Option<&Profile>;
    fn add(&mut self, profile: Profile) -> Result<(), ProfileError>;
    fn remove(&mut self, id: &str) -> Result<(), ProfileError>;
    fn set_default(&mut self, id: &str) -> Result<(), ProfileError>;
    fn default(&self) -> Option<&Profile>;
}

/// In-memory registry for tests and sim builds.
#[derive(Debug, Default, Clone)]
pub struct InMemoryRegistry {
    profiles: HashMap<String, Profile>,
    default_id: Option<String>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        <Self as Default>::default()
    }

    pub fn with_sample() -> Self {
        let mut r = Self::new();
        let _ = r.add(Profile {
            id: "aiserver-1".to_string(),
            label: "aiserver-1".to_string(),
            host: "aiserver-1.tailnet.ts.net".to_string(),
            port: 22,
            user: "pi".to_string(),
            key_path: "/home/pi/.ssh/id_m5tui".to_string(),
            jump_profile_id: None,
            omp_profile: "default".to_string(),
            default: true,
        });
        let _ = r.set_default("aiserver-1");
        r
    }
}

impl ProfileRegistry for InMemoryRegistry {
    fn list(&self) -> Vec<&Profile> {
        let mut out: Vec<&Profile> = self.profiles.values().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.get(id)
    }

    fn add(&mut self, profile: Profile) -> Result<(), ProfileError> {
        profile.validate()?;
        if self.profiles.contains_key(&profile.id) {
            return Err(ProfileError::Duplicate(profile.id));
        }
        if profile.default {
            self.default_id = Some(profile.id.clone());
        }
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    fn remove(&mut self, id: &str) -> Result<(), ProfileError> {
        if self.profiles.remove(id).is_none() {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        if self.default_id.as_deref() == Some(id) {
            self.default_id = None;
        }
        Ok(())
    }

    fn set_default(&mut self, id: &str) -> Result<(), ProfileError> {
        if !self.profiles.contains_key(id) {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        for p in self.profiles.values_mut() {
            p.default = false;
        }
        self.profiles
            .get_mut(id)
            .unwrap_or_else(|| unreachable!())
            .default = true;
        self.default_id = Some(id.to_string());
        Ok(())
    }

    fn default(&self) -> Option<&Profile> {
        self.default_id
            .as_ref()
            .and_then(|id| self.profiles.get(id))
    }
}

/// Parse a single profile YAML file into a [`Profile`].
pub fn parse_profile(yaml: &str) -> Result<Profile, ProfileError> {
    serde_yaml::from_str::<Profile>(yaml).map_err(|e| ProfileError::Yaml {
        line: e.location().map(|l| l.line()).unwrap_or(0),
        reason: e.to_string(),
    })
}

/// Parse a directory of `*.yaml` files using the given persistence driver.
/// Files that fail to parse are silently skipped; callers can layer logging
/// on top.
pub fn load_profile_dir(
    driver: &dyn m5tui_persist::Driver,
    dir: &str,
) -> Result<Vec<Profile>, ProfileError> {
    let mut out = Vec::new();
    for path in driver.list(dir)? {
        if !path.ends_with(".yaml") && !path.ends_with(".yml") {
            continue;
        }
        let bytes = driver.load(&path)?;
        let text = String::from_utf8(bytes).map_err(|e| ProfileError::Persist(e.to_string()))?;
        if let Ok(p) = parse_profile(&text) {
            if p.validate().is_ok() {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// Filesystem-backed profile registry with hot-reload support.
#[derive(Debug)]
#[allow(dead_code)]
pub struct FileRegistry {
    root: PathBuf,
    driver: m5tui_persist::FsDriver,
    profiles: HashMap<String, Profile>,
    default_id: Option<String>,
    mtimes: HashMap<String, SystemTime>,
}

impl FileRegistry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            root: root.clone(),
            driver: m5tui_persist::FsDriver::new(&root),
            profiles: HashMap::new(),
            default_id: None,
            mtimes: HashMap::new(),
        }
    }

    /// Scan the profile directory and refresh the in-memory index if any file
    /// has changed since the last scan. The `default_id` is derived from the
    /// `default: true` flag in YAML, falling back to the first profile.
    pub fn reload(&mut self) -> Result<(), ProfileError> {
        let dir = "/profiles";
        let paths = self.driver.list(dir)?;
        let mut changed = false;
        let mut seen = std::collections::HashSet::new();
        let mut first_id: Option<String> = None;

        for path in paths {
            if !path.ends_with(".yaml") && !path.ends_with(".yml") {
                continue;
            }
            seen.insert(path.clone());
            let current_mtime = self.driver.mtime(&path).ok();
            let prior = self.mtimes.get(&path).copied();
            if current_mtime == prior
                && self.profiles.values().any(|p| {
                    let file_id: String = path
                        .trim_start_matches("/profiles/")
                        .trim_end_matches(".yaml")
                        .trim_end_matches(".yml")
                        .to_string();
                    p.id == file_id
                })
            {
                continue;
            }

            let bytes = self.driver.load(&path)?;
            let text = String::from_utf8(bytes)
                .map_err(|e| ProfileError::Persist(format!("{path}: {e}")))?;
            let mut profile =
                parse_profile(&text).map_err(|e| ProfileError::Persist(format!("{path}: {e}")))?;
            profile.validate()?;
            if profile.id.is_empty() {
                profile.id = path
                    .trim_start_matches("/profiles/")
                    .trim_end_matches(".yaml")
                    .trim_end_matches(".yml")
                    .to_string();
            }
            if first_id.is_none() {
                first_id = Some(profile.id.clone());
            }
            self.profiles.insert(profile.id.clone(), profile);
            if let Some(t) = current_mtime {
                self.mtimes.insert(path.clone(), t);
            }
            changed = true;
        }

        // Drop profiles whose files disappeared.
        let to_remove: Vec<String> = self
            .profiles
            .keys()
            .filter(|id| {
                let path1 = format!("/profiles/{id}.yaml");
                let path2 = format!("/profiles/{id}.yml");
                !seen.contains(&path1) && !seen.contains(&path2)
            })
            .cloned()
            .collect();
        for id in &to_remove {
            self.profiles.remove(id);
            if self.default_id.as_deref() == Some(id) {
                self.default_id = None;
            }
            changed = true;
        }
        self.mtimes.retain(|p, _| seen.contains(p));

        // Recompute default from the `default` flag if the index changed.
        if changed {
            let mut new_default: Option<String> = None;
            for p in self.profiles.values() {
                if p.default {
                    new_default = Some(p.id.clone());
                    break;
                }
            }
            self.default_id = new_default.or(first_id);
        }

        Ok(())
    }

    /// Force reload of one file even if mtime has not changed.
    pub fn load_file(&mut self, filename: &str) -> Result<(), ProfileError> {
        let path = format!("/profiles/{filename}");
        let bytes = self.driver.load(&path)?;
        let text =
            String::from_utf8(bytes).map_err(|e| ProfileError::Persist(format!("{path}: {e}")))?;
        let mut profile =
            parse_profile(&text).map_err(|e| ProfileError::Persist(format!("{path}: {e}")))?;
        profile.validate()?;
        if profile.id.is_empty() {
            profile.id = filename
                .trim_end_matches(".yaml")
                .trim_end_matches(".yml")
                .to_string();
        }
        self.profiles.insert(profile.id.clone(), profile);
        if let Ok(t) = self.driver.mtime(&path) {
            self.mtimes.insert(path, t);
        }
        Ok(())
    }
}

impl ProfileRegistry for FileRegistry {
    fn list(&self) -> Vec<&Profile> {
        let mut out: Vec<&Profile> = self.profiles.values().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.get(id)
    }

    fn add(&mut self, profile: Profile) -> Result<(), ProfileError> {
        profile.validate()?;
        if self.profiles.contains_key(&profile.id) {
            return Err(ProfileError::Duplicate(profile.id));
        }
        let yaml =
            serde_yaml::to_string(&profile).map_err(|e| ProfileError::Persist(e.to_string()))?;
        let path = format!("/profiles/{}.yaml", profile.id);
        self.driver.save(&path, yaml.as_bytes())?;
        self.profiles.insert(profile.id.clone(), profile);
        if self.default_id.is_none() {
            self.default_id = Some(
                self.profiles
                    .keys()
                    .next()
                    .unwrap_or_else(|| unreachable!())
                    .clone(),
            );
        }
        Ok(())
    }

    fn remove(&mut self, id: &str) -> Result<(), ProfileError> {
        if self.profiles.remove(id).is_none() {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        let path = format!("/profiles/{id}.yaml");
        let _ = self.driver.delete(&path);
        self.mtimes.retain(|_, _| true);
        if self.default_id.as_deref() == Some(id) {
            self.default_id = self.profiles.keys().next().map(|k| k.to_string());
        }
        Ok(())
    }

    fn set_default(&mut self, id: &str) -> Result<(), ProfileError> {
        if !self.profiles.contains_key(id) {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        for p in self.profiles.values_mut() {
            p.default = false;
        }
        self.profiles
            .get_mut(id)
            .unwrap_or_else(|| unreachable!())
            .default = true;
        self.default_id = Some(id.to_string());
        Ok(())
    }

    fn default(&self) -> Option<&Profile> {
        self.default_id
            .as_ref()
            .and_then(|id| self.profiles.get(id))
    }
}

/// Resolve a profile into an ordered list of hosts from outermost hop to
/// target. This mirrors SSH `ProxyJump` semantics: a chain of intermediate
/// hosts ending in the final destination.
///
pub fn resolve_proxy_jump(
    registry: &dyn ProfileRegistry,
    start_id: &str,
) -> Result<Vec<ProxyHop>, ProfileError> {
    let mut hops = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut current_id = Some(start_id.to_string());

    while let Some(id) = current_id {
        if !seen.insert(id.clone()) {
            return Err(ProfileError::Invalid(format!(
                "proxy jump cycle detected involving {id}"
            )));
        }
        let profile = registry
            .get(&id)
            .ok_or_else(|| ProfileError::NotFound(id.clone()))?;
        hops.push(ProxyHop {
            id: profile.id.clone(),
            host: profile.host.clone(),
            port: profile.port,
            user: profile.user.clone(),
            key_path: profile.key_path.clone(),
        });
        current_id = profile.jump_profile_id.clone();
    }

    Ok(hops)
}

/// One hop in a `ProxyJump` chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyHop {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use m5tui_persist::MemoryDriver;
    use std::thread;
    use std::time::Duration;

    fn sample(id: &str) -> Profile {
        Profile {
            id: id.to_string(),
            label: "test".to_string(),
            host: "h".to_string(),
            port: 22,
            user: "u".to_string(),
            key_path: "k".to_string(),
            jump_profile_id: None,
            omp_profile: "omp".to_string(),
            default: false,
        }
    }

    #[test]
    fn registry_add_and_get() {
        let mut r = InMemoryRegistry::new();
        let p = sample("p1");
        r.add(p.clone()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.get("p1"), Some(&p));
    }

    #[test]
    fn registry_rejects_empty_id() {
        let mut r = InMemoryRegistry::new();
        let p = Profile {
            id: "".to_string(),
            ..sample("x")
        };
        assert!(r.add(p).is_err());
    }

    #[test]
    fn registry_default_round_trip() {
        let mut r = InMemoryRegistry::new();
        r.add(sample("p1")).unwrap_or_else(|e| panic!("{e}"));
        r.set_default("p1").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.default().map(|p| p.id.clone()), Some("p1".to_string()));
    }

    #[test]
    fn registry_remove_clears_default() {
        let mut r = InMemoryRegistry::with_sample();
        assert!(r.default().is_some());
        r.remove("aiserver-1").unwrap_or_else(|e| panic!("{e}"));
        assert!(r.default().is_none());
    }

    #[test]
    fn registry_list_sorted() {
        let mut r = InMemoryRegistry::new();
        r.add(sample("b")).unwrap_or_else(|e| panic!("{e}"));
        r.add(sample("a")).unwrap_or_else(|e| panic!("{e}"));
        let ids: Vec<String> = r.list().into_iter().map(|p| p.id.clone()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn yaml_parse_populates_fields() {
        let yaml = r#"
id: edge
label: Edge router
host: edge.tailnet.ts.net
port: 2222
user: admin
key_path: /home/pi/.ssh/edge
jump: bastion
omp_profile: edge
"#;
        let p = parse_profile(yaml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(p.id, "edge");
        assert_eq!(p.port, 2222);
        assert_eq!(p.jump_profile_id, Some("bastion".to_string()));
    }

    #[test]
    fn yaml_parse_uses_defaults() {
        let yaml = "id: tiny\nhost: tiny.local\nuser: pi\n";
        let p = parse_profile(yaml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(p.port, 22);
        assert_eq!(p.omp_profile, "");
        assert!(p.jump_profile_id.is_none());
    }

    #[test]
    fn yaml_validation_catches_missing_host() {
        let yaml = "id: bad\nuser: pi\n";
        let p = parse_profile(yaml).unwrap_or_else(|e| panic!("{e}"));
        assert!(p.validate().is_err());
    }

    #[test]
    fn load_profile_dir_reads_multiple_files() {
        let mut driver = MemoryDriver::new();
        let _ = driver.save("/profiles/a.yaml", b"id: a\nhost: a.local\nuser: u\n");
        let _ = driver.save("/profiles/b.yaml", b"id: b\nhost: b.local\nuser: u\n");
        let _ = driver.save("/profiles/ignore.txt", b"not yaml");
        let list = load_profile_dir(&driver, "/profiles").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|p| p.id == "a"));
        assert!(list.iter().any(|p| p.id == "b"));
    }

    #[test]
    fn file_registry_hot_reloads_on_mtime_change() {
        let tmp = std::env::temp_dir().join(format!("m5tui-profile-{}-hot", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let root = tmp.join("root");
        std::fs::create_dir_all(root.join("profiles")).unwrap_or_else(|e| panic!("{e}"));
        let yaml = "id: alpha\nhost: alpha.local\nuser: u\n";
        std::fs::write(root.join("profiles/alpha.yaml"), yaml).unwrap_or_else(|e| panic!("{e}"));

        let mut reg = FileRegistry::new(&root);
        reg.reload().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            reg.default().map(|p| p.id.clone()),
            Some("alpha".to_string())
        );

        thread::sleep(Duration::from_millis(50));
        let yaml2 = "id: beta\nhost: beta.local\nuser: u\ndefault: true\n";
        std::fs::write(root.join("profiles/beta.yaml"), yaml2).unwrap_or_else(|e| panic!("{e}"));
        reg.reload().unwrap_or_else(|e| panic!("{e}"));
        let ids: Vec<String> = reg.list().into_iter().map(|p| p.id.clone()).collect();
        assert_eq!(ids, vec!["alpha", "beta"]);
        assert_eq!(
            reg.default().map(|p| p.id.clone()),
            Some("beta".to_string())
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn file_registry_skips_reload_when_mtime_unchanged() {
        let tmp = std::env::temp_dir().join(format!("m5tui-profile-{}-skip", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let root = tmp.join("root");
        std::fs::create_dir_all(root.join("profiles")).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(root.join("profiles/x.yaml"), "id: x\nhost: x\nuser: u\n")
            .unwrap_or_else(|e| panic!("{e}"));

        let mut reg = FileRegistry::new(&root);
        reg.reload().unwrap_or_else(|e| panic!("{e}"));
        let reload2 = reg.reload();
        assert!(reload2.is_ok());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn proxy_jump_resolves_chain() {
        let mut r = InMemoryRegistry::new();
        r.add(Profile {
            id: "target".to_string(),
            jump_profile_id: Some("jump".to_string()),
            ..sample("target")
        })
        .unwrap_or_else(|e| panic!("{e}"));
        r.add(Profile {
            id: "jump".to_string(),
            host: "jump.ts.net".to_string(),
            jump_profile_id: None,
            ..sample("jump")
        })
        .unwrap_or_else(|e| panic!("{e}"));
        let hops = resolve_proxy_jump(&r, "target").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(hops.len(), 2);
        assert_eq!(hops[0].id, "target");
        assert_eq!(hops[1].id, "jump");
    }

    #[test]
    fn proxy_jump_without_jump_is_single_hop() {
        let mut r = InMemoryRegistry::new();
        r.add(sample("solo")).unwrap_or_else(|e| panic!("{e}"));
        let hops = resolve_proxy_jump(&r, "solo").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(hops.len(), 1);
        assert_eq!(hops[0].id, "solo");
    }

    #[test]
    fn proxy_jump_errors_on_missing_jump() {
        let mut r = InMemoryRegistry::new();
        r.add(Profile {
            id: "bad".to_string(),
            jump_profile_id: Some("missing".to_string()),
            ..sample("bad")
        })
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(resolve_proxy_jump(&r, "bad").is_err());
    }

    #[test]
    fn proxy_jump_detects_cycle() {
        let mut r = InMemoryRegistry::new();
        r.add(Profile {
            id: "a".to_string(),
            jump_profile_id: Some("b".to_string()),
            ..sample("a")
        })
        .unwrap_or_else(|e| panic!("{e}"));
        r.add(Profile {
            id: "b".to_string(),
            jump_profile_id: Some("a".to_string()),
            ..sample("b")
        })
        .unwrap_or_else(|e| panic!("{e}"));
        let err = resolve_proxy_jump(&r, "a").unwrap_err();
        assert!(matches!(err, ProfileError::Invalid(_)));
    }
}
