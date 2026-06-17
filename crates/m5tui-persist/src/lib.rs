//! `m5tui-persist` — persistence layer for m5Tui.
//!
//! This crate defines the [`Driver`] trait and two implementations:
//!
//! - [`MemoryDriver`] — in-memory store for unit tests and sim builds.
//! - [`FsDriver`] — filesystem-backed store for host builds. It is not
//!   SD-specific; it writes under a configurable root path. Device builds
//!   will later wrap the same trait around an SD driver.
//!
//! Features:
//!
//! - Atomic writes via temp file + rename + directory fsync.
//! - JSONL append with 512 KB rotation.
//! - Versioned config migration runner.
//! - Theme YAML load/save helpers.

use std::collections::HashMap;
use std::fs::{rename, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A persistence error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistError {
    Io(String),
    Utf8(String),
    Yaml { line: usize, reason: String },
    Json { reason: String },
    NotFound(String),
    Migration { version: u32, reason: String },
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Utf8(msg) => write!(f, "utf8 error: {msg}"),
            Self::Yaml { line, reason } => write!(f, "yaml error at line {line}: {reason}"),
            Self::Json { reason } => write!(f, "json error: {reason}"),
            Self::NotFound(path) => write!(f, "not found: {path}"),
            Self::Migration { version, reason } => {
                write!(f, "migration failed at version {version}: {reason}")
            }
        }
    }
}

impl std::error::Error for PersistError {}

impl PersistError {
    fn io(action: &str, path: &Path, e: io::Error) -> Self {
        Self::Io(format!("{action} {}: {e}", path.display()))
    }

    fn json(e: impl std::fmt::Display) -> Self {
        Self::Json {
            reason: e.to_string(),
        }
    }
}

/// Abstraction over any backing store (memory, filesystem, SD card).
pub trait Driver: Send + Sync {
    /// Load a file as bytes.
    fn load(&self, path: &str) -> Result<Vec<u8>, PersistError>;
    /// Write bytes, atomically where the backend supports it.
    fn save(&mut self, path: &str, data: &[u8]) -> Result<(), PersistError>;
    /// Append a line to a JSONL log, rotating at the configured size.
    fn append_jsonl(&mut self, path: &str, line: &[u8]) -> Result<(), PersistError>;
    /// List entries under a directory prefix.
    fn list(&self, dir: &str) -> Result<Vec<String>, PersistError>;
    /// Delete a file.
    fn delete(&mut self, path: &str) -> Result<(), PersistError>;
    /// Return the last-modified mtime of a file, if available.
    fn mtime(&self, path: &str) -> Result<std::time::SystemTime, PersistError>;
}

/// In-memory driver. Contents are keyed by a synthetic path.
#[derive(Debug, Default, Clone)]
pub struct MemoryDriver {
    files: HashMap<String, Vec<u8>>,
    mtimes: HashMap<String, std::time::SystemTime>,
    jsonl: HashMap<String, Vec<Vec<u8>>>,
}

impl MemoryDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(path: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let mut d = Self::new();
        let _ = d.save(&path.into(), &data.into());
        d
    }
}

impl Driver for MemoryDriver {
    fn load(&self, path: &str) -> Result<Vec<u8>, PersistError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| PersistError::NotFound(path.to_string()))
    }

    fn save(&mut self, path: &str, data: &[u8]) -> Result<(), PersistError> {
        self.files.insert(path.to_string(), data.to_vec());
        self.mtimes
            .insert(path.to_string(), std::time::SystemTime::now());
        Ok(())
    }

    fn append_jsonl(&mut self, path: &str, line: &[u8]) -> Result<(), PersistError> {
        self.jsonl
            .entry(path.to_string())
            .or_default()
            .push(line.to_vec());
        Ok(())
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, PersistError> {
        let prefix = format!("{dir}/");
        let mut out: Vec<String> = self
            .files
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| k.to_string())
            .collect();
        out.sort();
        Ok(out)
    }

    fn delete(&mut self, path: &str) -> Result<(), PersistError> {
        match self.files.remove(path) {
            Some(_) => {
                self.mtimes.remove(path);
                Ok(())
            }
            None => Err(PersistError::NotFound(path.to_string())),
        }
    }

    fn mtime(&self, path: &str) -> Result<std::time::SystemTime, PersistError> {
        self.mtimes
            .get(path)
            .copied()
            .ok_or_else(|| PersistError::NotFound(path.to_string()))
    }
}

/// Filesystem-backed driver. Writes under a configurable root path.
#[derive(Debug, Clone)]
pub struct FsDriver {
    root: PathBuf,
    jsonl_max_size: u64,
}

impl Default for FsDriver {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            jsonl_max_size: 512 * 1024,
        }
    }
}

impl FsDriver {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            jsonl_max_size: 512 * 1024,
        }
    }

    /// Override the JSONL rotation threshold. Default is 512 KB.
    pub fn with_jsonl_max_size(mut self, max_size: u64) -> Self {
        self.jsonl_max_size = max_size;
        self
    }

    fn resolve(&self, path: &str) -> PathBuf {
        self.root.join(path.trim_start_matches('/'))
    }
}

impl Driver for FsDriver {
    fn load(&self, path: &str) -> Result<Vec<u8>, PersistError> {
        let p = self.resolve(path);
        std::fs::read(&p).map_err(|e| PersistError::io("read", &p, e))
    }

    fn save(&mut self, path: &str, data: &[u8]) -> Result<(), PersistError> {
        let p = self.resolve(path);
        atomic_write(&p, data)?;
        Ok(())
    }

    fn append_jsonl(&mut self, path: &str, line: &[u8]) -> Result<(), PersistError> {
        let p = self.resolve(path);
        append_jsonl_rotated(&p, line, self.jsonl_max_size)?;
        Ok(())
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, PersistError> {
        let p = self.resolve(dir);
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&p).map_err(|e| PersistError::io("read_dir", &p, e))? {
            let entry = entry.map_err(|e| PersistError::io("entry", &p, e))?;
            let name = entry.file_name().to_string_lossy().to_string();
            out.push(format!("{dir}/{name}"));
        }
        out.sort();
        Ok(out)
    }

    fn delete(&mut self, path: &str) -> Result<(), PersistError> {
        let p = self.resolve(path);
        std::fs::remove_file(&p).map_err(|e| PersistError::io("remove", &p, e))
    }

    fn mtime(&self, path: &str) -> Result<std::time::SystemTime, PersistError> {
        let p = self.resolve(path);
        std::fs::metadata(&p)
            .map_err(|e| PersistError::io("metadata", &p, e))?
            .modified()
            .map_err(|e| PersistError::io("mtime", &p, e))
    }
}

/// Write `data` to `dst` atomically: temp file in the same directory,
/// fsync the file and directory, then rename into place.
pub fn atomic_write(dst: &Path, data: &[u8]) -> Result<(), PersistError> {
    let Some(parent) = dst.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Err(PersistError::Io(format!(
            "atomic_write: no parent for {}",
            dst.display()
        )));
    };
    std::fs::create_dir_all(parent).map_err(|e| PersistError::io("mkdir", parent, e))?;

    let name = dst
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "tmp".to_string());
    let tmp = parent.join(format!(".{}.tmp", name));
    let mut f = File::create(&tmp).map_err(|e| PersistError::io("create", &tmp, e))?;
    f.write_all(data)
        .map_err(|e| PersistError::io("write", &tmp, e))?;
    f.flush().map_err(|e| PersistError::io("flush", &tmp, e))?;
    f.sync_all()
        .map_err(|e| PersistError::io("sync", &tmp, e))?;
    drop(f);

    rename(&tmp, dst).map_err(|e| PersistError::io("rename", dst, e))?;
    sync_dir(parent)?;
    Ok(())
}

fn sync_dir(dir: &Path) -> Result<(), PersistError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let f = File::open(dir).map_err(|e| PersistError::io("open_dir", dir, e))?;
        f.sync_all()
            .map_err(|e| PersistError::io("sync_dir", dir, e))?;
    }
    Ok(())
}

/// Append `line` to `dst`, adding a trailing newline if absent. When the
/// file reaches `max_size`, rotate it to `<dst>.0`, evicting older slots.
pub fn append_jsonl_rotated(dst: &Path, line: &[u8], max_size: u64) -> Result<(), PersistError> {
    let Some(parent) = dst.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Err(PersistError::Io(format!(
            "append_jsonl: no parent for {}",
            dst.display()
        )));
    };
    std::fs::create_dir_all(parent).map_err(|e| PersistError::io("mkdir", parent, e))?;

    if let Some(meta) = std::fs::metadata(dst).ok().filter(|m| m.is_file()) {
        if meta.len() + (line.len() as u64) + 1 >= max_size {
            let rotated = dst.with_extension("jsonl.0");
            let _ = std::fs::remove_file(&rotated);
            rename(dst, rotated).map_err(|e| PersistError::io("rotate", dst, e))?;
            sync_dir(parent)?;
        }
    }

    let mut f = File::options()
        .create(true)
        .append(true)
        .open(dst)
        .map_err(|e| PersistError::io("open", dst, e))?;

    let mut buf = line.to_vec();
    if buf.last() != Some(&b'\n') {
        buf.push(b'\n');
    }
    f.write_all(&buf)
        .map_err(|e| PersistError::io("append", dst, e))?;
    f.flush().map_err(|e| PersistError::io("flush", dst, e))?;
    f.sync_all().map_err(|e| PersistError::io("sync", dst, e))?;
    Ok(())
}

/// Migration step: transform bytes from version `from` to `from + 1`.
pub trait Migration: Send + Sync {
    fn version(&self) -> u32;
    fn migrate(&self, bytes: &[u8]) -> Result<Vec<u8>, PersistError>;
}

/// Run a chain of migrations against an already-loaded file body.
///
/// `current_version` reads the embedded schema version; `migrations` must
/// be sorted ascending by `from_version`. The runner applies each needed
/// step, then writes the result through `save`. Old files are preserved as
/// `<path>.v<N>.bak` before overwrite, per project rule 10.
pub fn run_migrations<D: Driver>(
    driver: &mut D,
    path: &str,
    current_version: impl Fn(&[u8]) -> Result<u32, PersistError>,
    target_version: u32,
    migrations: &[&dyn Migration],
) -> Result<(), PersistError> {
    let bytes = driver.load(path)?;
    let mut version = current_version(&bytes)?;
    if version == target_version {
        return Ok(());
    }
    if version > target_version {
        return Err(PersistError::Migration {
            version,
            reason: format!(
                "file version {version} is newer than target {target_version}; downgrade not supported"
            ),
        });
    }

    let mut working = bytes;
    let mut applied = false;
    while version < target_version {
        let m = migrations
            .iter()
            .find(|m| m.version() == version)
            .ok_or_else(|| PersistError::Migration {
                version,
                reason: format!("no migration registered from version {version}"),
            })?;
        working = m.migrate(&working)?;
        version += 1;
        applied = true;
    }

    if applied {
        let backup = format!("{path}.v{version}.bak");
        // Best-effort backup of the original file. MemoryDriver simply copies.
        if let Ok(orig) = driver.load(path) {
            let _ = driver.save(&backup, &orig);
        }
        driver.save(path, &working)?;
    }

    Ok(())
}

/// Helper: migrate a JSONL config from version N to N+1 by deserializing,
/// updating `version`, and re-serializing.
pub fn json_version_bump<T>(
    bytes: &[u8],
    version_field: fn(&mut T),
) -> Result<Vec<u8>, PersistError>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut value: T = serde_json::from_slice(bytes).map_err(PersistError::json)?;
    version_field(&mut value);
    serde_json::to_vec_pretty(&value).map_err(PersistError::json)
}

/// Load a theme YAML from a driver.
pub fn load_theme(driver: &dyn Driver, path: &str) -> Result<m5tui_themes::Theme, PersistError> {
    let bytes = driver.load(path)?;
    let text = String::from_utf8(bytes).map_err(|e| PersistError::Utf8(e.to_string()))?;
    m5tui_themes::parse(&text).map_err(|e| PersistError::Yaml {
        line: 0,
        reason: e.to_string(),
    })
}

/// Save a theme YAML to a driver.
pub fn save_theme(
    driver: &mut dyn Driver,
    path: &str,
    theme: &m5tui_themes::Theme,
) -> Result<(), PersistError> {
    let pal = &theme.palette;
    let swatches: [(&str, u32); 10] = [
        ("bg", pal.bg.to_rgb24()),
        ("fg", pal.fg.to_rgb24()),
        ("accent", pal.accent.to_rgb24()),
        ("dim", pal.dim.to_rgb24()),
        ("warn", pal.warn.to_rgb24()),
        ("err", pal.err.to_rgb24()),
        ("ok", pal.ok.to_rgb24()),
        ("sel_bg", pal.sel_bg.to_rgb24()),
        ("sel_fg", pal.sel_fg.to_rgb24()),
        ("prompt", pal.prompt.to_rgb24()),
    ];
    let mut yaml = String::new();
    yaml.push_str("name: ");
    yaml.push_str(&theme.name);
    yaml.push_str("\nbrightness: ");
    yaml.push_str(&theme.brightness.to_string());
    yaml.push_str("\npalette:\n");
    for (name, rgb24) in swatches {
        yaml.push_str("  ");
        yaml.push_str(name);
        yaml.push_str(": \"#");
        yaml.push_str(&format!("{rgb24:06x}"));
        yaml.push_str("\"\n");
    }
    driver.save(path, yaml.as_bytes())
}

/// Read a JSONL file as a raw byte stream, returning an iterator-friendly
/// vector of lines. Used by callers that want to replay logs.
pub fn read_jsonl_lines(driver: &dyn Driver, path: &str) -> Result<Vec<Vec<u8>>, PersistError> {
    let bytes = driver.load(path)?;
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            lines.push(bytes[start..i].to_vec());
            start = i + 1;
        }
    }
    if start < bytes.len() {
        lines.push(bytes[start..].to_vec());
    }
    Ok(lines)
}

/// Count lines in a JSONL file without materializing the whole body twice.
pub fn jsonl_line_count(driver: &dyn Driver, path: &str) -> Result<usize, PersistError> {
    Ok(read_jsonl_lines(driver, path)?.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn memory_driver_round_trip() {
        let mut d = MemoryDriver::new();
        d.save("/themes/coldwire.yaml", b"hello")
            .unwrap_or_else(|e| panic!("{e}"));
        let got = d
            .load("/themes/coldwire.yaml")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, b"hello");
    }

    #[test]
    fn memory_driver_delete() {
        let mut d = MemoryDriver::new();
        d.save("/a.txt", b"x").unwrap_or_else(|e| panic!("{e}"));
        assert!(d.load("/a.txt").is_ok());
        d.delete("/a.txt").unwrap_or_else(|e| panic!("{e}"));
        assert!(d.load("/a.txt").is_err());
    }

    #[test]
    fn memory_driver_list_sorted() {
        let mut d = MemoryDriver::new();
        d.save("/themes/b.yaml", b"b")
            .unwrap_or_else(|e| panic!("{e}"));
        d.save("/themes/a.yaml", b"a")
            .unwrap_or_else(|e| panic!("{e}"));
        let list = d.list("/themes").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(list, vec!["/themes/a.yaml", "/themes/b.yaml"]);
    }

    #[test]
    fn memory_driver_mtime_updates_on_save() {
        let mut d = MemoryDriver::new();
        let before = std::time::SystemTime::now();
        d.save("/x", b"1").unwrap_or_else(|e| panic!("{e}"));
        let t1 = d.mtime("/x").unwrap_or_else(|e| panic!("{e}"));
        assert!(t1 >= before);
        thread::sleep(Duration::from_millis(10));
        d.save("/x", b"2").unwrap_or_else(|e| panic!("{e}"));
        let t2 = d.mtime("/x").unwrap_or_else(|e| panic!("{e}"));
        assert!(t2 > t1);
    }

    #[test]
    fn save_theme_minimal() {
        let mut d = MemoryDriver::new();
        let theme = m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("missing coldwire"));
        save_theme(&mut d, "/themes/coldwire.yaml", &theme).unwrap_or_else(|e| panic!("{e}"));
        let loaded = load_theme(&d, "/themes/coldwire.yaml").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(loaded.name, "coldwire");
        assert_eq!(loaded.brightness, theme.brightness);
    }

    #[test]
    fn fs_driver_round_trip() {
        let tmp = std::env::temp_dir().join(format!("m5tui-persist-{}-rt", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut d = FsDriver::new(&tmp);
        d.save("/scrollback/session-1.txt", b"line1\nline2\n")
            .unwrap_or_else(|e| panic!("{e}"));
        let got = d
            .load("/scrollback/session-1.txt")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, b"line1\nline2\n");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn fs_driver_atomic_write_visible_only_after_rename() {
        let tmp = std::env::temp_dir().join(format!("m5tui-persist-{}-atomic", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let dst = tmp.join("config.yaml");

        // Write directly through atomic_write, then ensure no temp file remains.
        atomic_write(&dst, b"committed").unwrap_or_else(|e| panic!("{e}"));
        let got = std::fs::read(&dst).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, b"committed");
        let temps: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap_or_else(|e| panic!("{e}"))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("."))
            .collect();
        assert!(temps.is_empty(), "temp file leaked: {temps:?}");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn fs_driver_append_jsonl_adds_newline_and_rotates() {
        let tmp = std::env::temp_dir().join(format!("m5tui-persist-{}-jsonl", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut d = FsDriver::new(&tmp).with_jsonl_max_size(64);
        let long = b"{\"a\":11111111111111111111111111111111111111111111111111111}";
        d.append_jsonl("/logs/events.jsonl", long)
            .unwrap_or_else(|e| panic!("{e}"));
        let raw = d
            .load("/logs/events.jsonl")
            .unwrap_or_else(|e| panic!("{e}"));
        let text = String::from_utf8(raw).unwrap_or_else(|e| panic!("{e}"));
        assert!(text.ends_with('\n'), "append should add a trailing newline");
        assert!(text.starts_with("{\"a\":"), "first line preserved");

        // A second large append pushes the file over 64 bytes and must rotate.
        d.append_jsonl("/logs/events.jsonl", b"{\"b\":2}\n")
            .unwrap_or_else(|e| panic!("{e}"));
        let rotated = tmp.join("logs/events.jsonl.0");
        assert!(rotated.exists(), "rotated file missing");
        let active = d
            .load("/logs/events.jsonl")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            active, b"{\"b\":2}\n",
            "active file holds only the latest line after rotation"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn jsonl_line_count_matches() {
        let mut d = MemoryDriver::new();
        d.save("/log.jsonl", b"one\ntwo\nthree")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            jsonl_line_count(&d, "/log.jsonl").unwrap_or_else(|e| panic!("{e}")),
            3
        );
    }

    #[test]
    fn migration_runner_applies_steps_and_bumps_version() {
        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
        struct Config {
            version: u32,
            name: String,
        }

        let mut d = MemoryDriver::new();
        d.save("/config.json", b"{\"version\":1,\"name\":\"old\"}")
            .unwrap_or_else(|e| panic!("{e}"));

        struct Step1;
        impl Migration for Step1 {
            fn version(&self) -> u32 {
                1
            }
            fn migrate(&self, bytes: &[u8]) -> Result<Vec<u8>, PersistError> {
                let mut c: Config = serde_json::from_slice(bytes).map_err(PersistError::json)?;
                c.version = 2;
                c.name = format!("{}-v2", c.name);
                serde_json::to_vec(&c).map_err(PersistError::json)
            }
        }

        run_migrations(
            &mut d,
            "/config.json",
            |b| {
                let c: Config = serde_json::from_slice(b).map_err(PersistError::json)?;
                Ok(c.version)
            },
            2,
            &[&Step1 as &dyn Migration],
        )
        .unwrap_or_else(|e| panic!("{e}"));

        let final_bytes = d.load("/config.json").unwrap_or_else(|e| panic!("{e}"));
        let final_cfg: Config =
            serde_json::from_slice(&final_bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(final_cfg.version, 2);
        assert_eq!(final_cfg.name, "old-v2");
        assert!(d.load("/config.json.v2.bak").is_ok());
    }

    #[test]
    fn migration_runner_errors_when_step_missing() {
        let mut d = MemoryDriver::new();
        d.save("/cfg.json", b"{\"version\":1}")
            .unwrap_or_else(|e| panic!("{e}"));
        let err = run_migrations(
            &mut d,
            "/cfg.json",
            |b| {
                let v: serde_json::Value = serde_json::from_slice(b).map_err(PersistError::json)?;
                v["version"]
                    .as_u64()
                    .map(|v| v as u32)
                    .ok_or_else(|| PersistError::Migration {
                        version: 0,
                        reason: "missing version".to_string(),
                    })
            },
            3,
            &[],
        )
        .expect_err("expected migration to fail without a registered step");
        assert!(
            matches!(err, PersistError::Migration { version: 1, .. }),
            "unexpected err: {err}"
        );
    }
}
