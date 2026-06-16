//! `m5tui-persist` — persistence layer for m5Tui.
//!
//! This crate defines the `PersistDriver` trait and two implementations:
//!
//! - `MemoryDriver` — in-memory store for unit tests and sim builds.
//! - `FileDriver` — filesystem-backed store for host builds (device builds
//!   will later map this to an SD driver).
//!
//! M3 scope is intentionally narrow: load/save YAML themes, scrollback
//! chunks, and profile registry files. Real SD-card atomic writes and
//! migration are stubbed with clear TODO markers for the hardware phase.

use std::collections::HashMap;

/// A persistence error. The `Io` variant wraps a string because `std::io`
/// is not available on every no_std target we may later support; the host
/// impl can map `io::Error` to this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistError {
    Io(String),
    Utf8(String),
    Yaml { line: usize, reason: String },
    NotFound(String),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Utf8(msg) => write!(f, "utf8 error: {msg}"),
            Self::Yaml { line, reason } => write!(f, "yaml error at line {line}: {reason}"),
            Self::NotFound(path) => write!(f, "not found: {path}"),
        }
    }
}

impl std::error::Error for PersistError {}

/// Abstraction over any backing store (memory, filesystem, SD card).
pub trait PersistDriver: Send + Sync {
    /// Load a file as bytes.
    fn load(&self, path: &str) -> Result<Vec<u8>, PersistError>;
    /// Write bytes, atomically where the backend supports it.
    fn save(&mut self, path: &str, data: &[u8]) -> Result<(), PersistError>;
    /// List entries under a directory prefix.
    fn list(&self, dir: &str) -> Result<Vec<String>, PersistError>;
    /// Delete a file.
    fn delete(&mut self, path: &str) -> Result<(), PersistError>;
}

/// In-memory driver. Contents are keyed by a synthetic path.
#[derive(Debug, Default, Clone)]
pub struct MemoryDriver {
    files: HashMap<String, Vec<u8>>,
}

impl MemoryDriver {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PersistDriver for MemoryDriver {
    fn load(&self, path: &str) -> Result<Vec<u8>, PersistError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| PersistError::NotFound(path.to_string()))
    }

    fn save(&mut self, path: &str, data: &[u8]) -> Result<(), PersistError> {
        self.files.insert(path.to_string(), data.to_vec());
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
            Some(_) => Ok(()),
            None => Err(PersistError::NotFound(path.to_string())),
        }
    }
}

/// Host filesystem driver. Not used on device; maps to `std::fs`.
#[derive(Debug, Default, Clone)]
pub struct FileDriver {
    root: std::path::PathBuf,
}

impl FileDriver {
    pub fn new(root: impl AsRef<std::path::Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn resolve(&self, path: &str) -> std::path::PathBuf {
        self.root.join(path.trim_start_matches('/'))
    }
}

impl PersistDriver for FileDriver {
    fn load(&self, path: &str) -> Result<Vec<u8>, PersistError> {
        let p = self.resolve(path);
        std::fs::read(&p).map_err(|e| PersistError::Io(format!("read {}: {}", p.display(), e)))
    }

    fn save(&mut self, path: &str, data: &[u8]) -> Result<(), PersistError> {
        let p = self.resolve(path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PersistError::Io(format!("mkdir {}: {}", parent.display(), e)))?;
        }
        std::fs::write(&p, data)
            .map_err(|e| PersistError::Io(format!("write {}: {}", p.display(), e)))
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, PersistError> {
        let p = self.resolve(dir);
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&p)
            .map_err(|e| PersistError::Io(format!("read_dir {}: {}", p.display(), e)))?
        {
            let entry = entry.map_err(|e| PersistError::Io(format!("entry: {e}")))?;
            let name = entry.file_name().to_string_lossy().to_string();
            out.push(format!("{dir}/{name}"));
        }
        out.sort();
        Ok(out)
    }

    fn delete(&mut self, path: &str) -> Result<(), PersistError> {
        let p = self.resolve(path);
        std::fs::remove_file(&p)
            .map_err(|e| PersistError::Io(format!("remove {}: {}", p.display(), e)))
    }
}

/// Load a theme YAML from a driver.
pub fn load_theme(
    driver: &dyn PersistDriver,
    path: &str,
) -> Result<m5tui_themes::Theme, PersistError> {
    let bytes = driver.load(path)?;
    let text = String::from_utf8(bytes).map_err(|e| PersistError::Utf8(e.to_string()))?;
    m5tui_themes::parse(&text).map_err(|e| PersistError::Yaml {
        line: 0,
        reason: e.to_string(),
    })
}

/// Save a theme YAML to a driver.
pub fn save_theme(
    driver: &mut dyn PersistDriver,
    path: &str,
    theme: &m5tui_themes::Theme,
) -> Result<(), PersistError> {
    // Persist a fully valid theme YAML so it can be re-loaded by the
    // m5tui-themes parser. Real serialization would emit all sections.
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
    yaml.push_str(
        "
brightness: ",
    );
    yaml.push_str(&theme.brightness.to_string());
    yaml.push_str(
        "
palette:
",
    );
    for (name, rgb24) in swatches {
        yaml.push_str("  ");
        yaml.push_str(name);
        yaml.push_str(": \"#");
        yaml.push_str(&format!("{rgb24:06x}"));
        yaml.push_str("\"\n");
    }
    driver.save(path, yaml.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn save_theme_minimal() {
        let mut d = MemoryDriver::new();
        let theme = m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("missing coldwire"));
        save_theme(&mut d, "/themes/coldwire.yaml", &theme).unwrap_or_else(|e| panic!("{e}"));
        let loaded = load_theme(&d, "/themes/coldwire.yaml").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(loaded.name, "coldwire");
        assert_eq!(loaded.brightness, theme.brightness);
    }

    #[test]
    fn file_driver_round_trip() {
        let tmp = std::env::temp_dir().join(format!("m5tui-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut d = FileDriver::new(&tmp);
        d.save(
            "/scrollback/session-1.txt",
            b"line1
line2
",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let got = d
            .load("/scrollback/session-1.txt")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            got,
            b"line1
line2
"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
