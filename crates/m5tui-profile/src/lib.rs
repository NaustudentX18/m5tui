//! `m5tui-profile` — connection profile registry.
//!
//! A `Profile` bundles everything needed to reach a remote host: SSH
//! credentials, an optional jump host, and an OMP profile name. The
//! `ProfileRegistry` trait abstracts storage; `InMemoryRegistry` is the
//! default for tests and sim builds.

use std::collections::HashMap;

/// A connection profile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub host: String,
    pub user: String,
    /// Path to an SSH private key or empty for agent-based auth.
    pub key_path: String,
    /// Optional jump-host profile id.
    pub jump_profile_id: Option<String>,
    /// OMP profile name used by the orchestrator sidecar.
    pub omp_profile: String,
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
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(msg) => write!(f, "invalid profile: {msg}"),
            Self::NotFound(id) => write!(f, "profile not found: {id}"),
            Self::Duplicate(id) => write!(f, "duplicate profile: {id}"),
            Self::Persist(msg) => write!(f, "persist error: {msg}"),
        }
    }
}

impl std::error::Error for ProfileError {}

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
            user: "pi".to_string(),
            key_path: "/home/pi/.ssh/id_m5tui".to_string(),
            jump_profile_id: None,
            omp_profile: "default".to_string(),
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
        self.default_id = Some(id.to_string());
        Ok(())
    }

    fn default(&self) -> Option<&Profile> {
        self.default_id
            .as_ref()
            .and_then(|id| self.profiles.get(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> Profile {
        Profile {
            id: id.to_string(),
            label: "test".to_string(),
            host: "h".to_string(),
            user: "u".to_string(),
            key_path: "k".to_string(),
            jump_profile_id: None,
            omp_profile: "omp".to_string(),
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
}
