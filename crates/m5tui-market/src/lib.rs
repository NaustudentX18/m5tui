//! `m5tui-market` — community theme market.
//!
//! M5b scope: catalog schema, a stub HTTPS-like client with canned data,
//! offline cache, and live theme preview via `m5tui-themes::parse`.
//! A real HTTPS client and GitHub Pages/R2 backend are deferred.

use std::collections::HashMap;

/// A theme entry in the market catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Author,
    pub description: String,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Author {
    pub name: String,
    pub url: String,
}

/// The market catalog.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Catalog {
    pub version: u32,
    pub entries: Vec<ThemeEntry>,
}

impl Catalog {
    pub fn find(&self, id: &str) -> Option<&ThemeEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn list(&self) -> &[ThemeEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketError {
    Network(String),
    Catalog(String),
    Theme(String),
    NotFound(String),
}

impl std::fmt::Display for MarketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(s) => write!(f, "network: {s}"),
            Self::Catalog(s) => write!(f, "catalog: {s}"),
            Self::Theme(s) => write!(f, "theme: {s}"),
            Self::NotFound(s) => write!(f, "not found: {s}"),
        }
    }
}

impl std::error::Error for MarketError {}

/// Market client abstraction.
pub trait MarketClient: Send + Sync {
    fn fetch_catalog(&self) -> Result<Catalog, MarketError>;
    fn download_theme(&mut self, id: &str) -> Result<String, MarketError>;
    fn preview_theme(&self, yaml: &str) -> Result<m5tui_themes::Theme, MarketError>;
}

/// Parse the simple line-oriented catalog format used by the stub.
pub fn parse_catalog(text: &str) -> Result<Catalog, MarketError> {
    let mut entries = Vec::new();
    let mut lines = text.lines();
    let version_line = lines
        .next()
        .ok_or_else(|| MarketError::Catalog("empty catalog".to_string()))?;
    let version: u32 = version_line
        .strip_prefix("version:")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| MarketError::Catalog(format!("bad version line: {version_line}")))?;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 6 {
            return Err(MarketError::Catalog(format!("bad entry: {line}")));
        }
        entries.push(ThemeEntry {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            version: parts[2].to_string(),
            author: Author {
                name: parts[3].to_string(),
                url: parts[4].to_string(),
            },
            download_url: parts[5].to_string(),
            description: "".to_string(),
        });
    }
    Ok(Catalog { version, entries })
}

/// Serialize the catalog to the simple line-oriented format.
pub fn serialize_catalog(catalog: &Catalog) -> String {
    let mut out = format!(
        "version: {}
",
        catalog.version
    );
    for e in &catalog.entries {
        out.push_str(&format!(
            "{}|{}|{}|{}|{}|{}
",
            e.id, e.name, e.version, e.author.name, e.author.url, e.download_url
        ));
    }
    out
}

/// Stub client backed by canned catalog strings and an in-memory store.
pub struct StubMarketClient {
    catalog: Catalog,
    store: HashMap<String, String>,
}

impl StubMarketClient {
    pub fn with_canned() -> Self {
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let mut store = HashMap::new();
        store.insert("coldwire.yaml".to_string(), yaml.to_string());
        Self {
            catalog: Catalog {
                version: 1,
                entries: vec![ThemeEntry {
                    id: "coldwire.yaml".to_string(),
                    name: "Coldwire".to_string(),
                    version: "0.2.0".to_string(),
                    author: Author {
                        name: "Forest Hudson".to_string(),
                        url: "https://github.com/NaustudentX18".to_string(),
                    },
                    description: "Hero theme".to_string(),
                    download_url: "https://example.com/coldwire.yaml".to_string(),
                }],
            },
            store,
        }
    }
}

impl MarketClient for StubMarketClient {
    fn fetch_catalog(&self) -> Result<Catalog, MarketError> {
        Ok(self.catalog.clone())
    }

    fn download_theme(&mut self, id: &str) -> Result<String, MarketError> {
        self.store
            .get(id)
            .cloned()
            .ok_or_else(|| MarketError::NotFound(id.to_string()))
    }

    fn preview_theme(&self, yaml: &str) -> Result<m5tui_themes::Theme, MarketError> {
        m5tui_themes::parse(yaml).map_err(|e| MarketError::Theme(e.to_string()))
    }
}

/// Offline cache: write/read the catalog text.
pub struct OfflineCache;

impl Default for OfflineCache {
    fn default() -> Self {
        Self
    }
}

impl OfflineCache {
    pub fn new() -> Self {
        Self
    }

    pub fn save(&self, _path: &str, text: &str) -> Result<(), MarketError> {
        // M5b stub: in-memory only; real filesystem cache deferred.
        let _ = text;
        Ok(())
    }

    pub fn load(&self, _path: &str) -> Result<String, MarketError> {
        Err(MarketError::Catalog(
            "offline cache not implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_canned_catalog() {
        let text = "version: 1
coldwire.yaml|Coldwire|0.2.0|Forest Hudson|https://github.com/NaustudentX18|https://example.com/coldwire.yaml
";
        let c = parse_catalog(text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.version, 1);
        assert_eq!(c.entries.len(), 1);
        assert_eq!(c.entries[0].name, "Coldwire");
    }

    #[test]
    fn serialize_and_parse_round_trip() {
        let c = Catalog {
            version: 2,
            entries: vec![ThemeEntry {
                id: "phosphor.yaml".to_string(),
                name: "Phosphor".to_string(),
                version: "1.0.0".to_string(),
                author: Author {
                    name: "A".to_string(),
                    url: "https://a".to_string(),
                },
                description: "".to_string(),
                download_url: "https://a/phosphor.yaml".to_string(),
            }],
        };
        let text = serialize_catalog(&c);
        let parsed = parse_catalog(&text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(parsed, c);
    }

    #[test]
    fn stub_fetch_catalog() {
        let client = StubMarketClient::with_canned();
        let c = client.fetch_catalog().unwrap_or_else(|e| panic!("{e}"));
        assert!(!c.entries.is_empty());
    }

    #[test]
    fn stub_download_known_theme() {
        let mut client = StubMarketClient::with_canned();
        let yaml = client
            .download_theme("coldwire.yaml")
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(yaml.contains("name: coldwire"));
    }

    #[test]
    fn stub_preview_builtin_theme() {
        let client = StubMarketClient::with_canned();
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let theme = client.preview_theme(yaml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(theme.name, "coldwire");
    }
}
