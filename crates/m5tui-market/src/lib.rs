//! `m5tui-market` — community theme market.
//!
//! M5b scope: catalog schema, HTTPS catalog fetch via `ureq`, offline cache
//! backed by a `m5tui-persist::Driver`, theme install/preview/publish,
//! and live cockpit preview via `m5tui-core::render`.

use std::collections::HashMap;
use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

/// Path to the offline catalog cache inside a `Driver` root.
pub const CATALOG_CACHE_PATH: &str = "market/catalog.json";

/// Default install directory for themes inside a `Driver` root.
pub const THEMES_DIR: &str = "themes";

/// A theme entry in the market catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeEntry {
    /// Stable theme identifier, e.g. `coldwire+`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Semver-ish version string.
    pub version: String,
    /// Author handle.
    pub author: Author,
    /// One-line description.
    #[serde(default)]
    pub description: String,
    /// SHA-256 of the canonical theme YAML asset.
    #[serde(default)]
    pub sha256: String,
    /// Size of the asset in bytes.
    #[serde(default)]
    pub size_bytes: u64,
    /// Rating rendered as a string (e.g. `4.9`) to keep the type `Eq`.
    #[serde(default)]
    pub stars: String,
    /// Install / download count.
    #[serde(default)]
    pub installs: u64,
    /// Three-hex palette swatch for list rows.
    #[serde(default)]
    pub preview_swatch: Vec<String>,
    /// OMP versions the theme has been tested against.
    #[serde(default)]
    pub tested_omp_versions: Vec<String>,
    /// Discovery tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// HTTPS URL of the theme YAML asset.
    #[serde(default)]
    pub download_url: String,
}

/// Theme author. The catalog format usually carries a handle string, so the
/// JSON deserializer accepts either `"forest"` or `{"name":"forest","url":"…"}`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Author {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
}

impl Author {
    /// Build an author from a handle, with an empty URL.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: String::new(),
        }
    }

    /// Build an author with a handle and optional homepage.
    pub fn with_url(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
        }
    }
}

impl<'de> Deserialize<'de> for Author {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AuthorVisitor;

        impl<'de> de::Visitor<'de> for AuthorVisitor {
            type Value = Author;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string or object with name and optional url")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Author::new(value))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut name: Option<String> = None;
                let mut url: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = Some(map.next_value()?),
                        "url" => url = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>();
                        }
                    }
                }
                Ok(Author {
                    name: name.unwrap_or_default(),
                    url: url.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_any(AuthorVisitor)
    }
}

/// The market catalog.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub version: u32,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default, rename = "themes")]
    pub entries: Vec<ThemeEntry>,
}

impl Catalog {
    /// Find an entry by id.
    pub fn find(&self, id: &str) -> Option<&ThemeEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// List all entries.
    pub fn list(&self) -> &[ThemeEntry] {
        &self.entries
    }
}

/// Market operation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketError {
    Network(String),
    Catalog(String),
    Theme(String),
    NotFound(String),
    Persist(String),
    Publish(String),
}

impl fmt::Display for MarketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(s) => write!(f, "network: {s}"),
            Self::Catalog(s) => write!(f, "catalog: {s}"),
            Self::Theme(s) => write!(f, "theme: {s}"),
            Self::NotFound(s) => write!(f, "not found: {s}"),
            Self::Persist(s) => write!(f, "persist: {s}"),
            Self::Publish(s) => write!(f, "publish: {s}"),
        }
    }
}

impl std::error::Error for MarketError {}

/// Market client abstraction.
pub trait MarketClient: Send + Sync {
    /// Download and parse the remote catalog.
    fn fetch_catalog(&self) -> Result<Catalog, MarketError>;
    /// Download the theme YAML for the given entry id.
    fn download_theme(&mut self, id: &str) -> Result<String, MarketError>;
    /// Parse a theme YAML into a validated `Theme`.
    fn preview_theme(&self, yaml: &str) -> Result<m5tui_themes::Theme, MarketError>;
}

/// Parse a catalog from its JSON representation.
pub fn parse_catalog(text: &str) -> Result<Catalog, MarketError> {
    serde_json::from_str(text).map_err(|e| MarketError::Catalog(e.to_string()))
}

/// Serialize a catalog to pretty-printed JSON.
pub fn serialize_catalog(catalog: &Catalog) -> Result<String, MarketError> {
    serde_json::to_string_pretty(catalog).map_err(|e| MarketError::Catalog(e.to_string()))
}

/// Parse and validate a theme YAML.
pub fn preview_theme(yaml: &str) -> Result<m5tui_themes::Theme, MarketError> {
    m5tui_themes::parse_validated(yaml).map_err(|e| MarketError::Theme(e.to_string()))
}

/// Render a sample cockpit with the previewed theme.
pub fn render_preview(theme: &m5tui_themes::Theme) -> m5tui_core::Frame {
    let state = m5tui_core::AppState::default();
    m5tui_core::render(&state, theme)
}

/// Install a theme YAML into the persist store under `themes/{id}.yaml`.
/// Idempotent: writing the same content twice is a no-op.
pub fn install_theme(
    driver: &mut dyn m5tui_persist::Driver,
    id: &str,
    yaml: &str,
) -> Result<(), MarketError> {
    let _theme = preview_theme(yaml)?;
    let path = format!("{THEMES_DIR}/{id}.yaml");
    driver
        .save(&path, yaml.as_bytes())
        .map_err(|e| MarketError::Persist(e.to_string()))
}

/// Everything produced by a publish step. The caller can upload `asset_yaml`
/// to `asset_path` and append `entry` to the catalog; no backend I/O happens
/// here, so tests stay offline.
pub struct PublishDraft {
    pub entry: ThemeEntry,
    pub asset_path: String,
    pub asset_yaml: String,
    pub sha256: String,
}

/// Build a catalog entry and asset for a theme without touching the network.
///
/// `sha256` and `size_bytes` are computed from the canonical YAML produced by
/// this crate, and `preview_swatch` is derived from the theme palette.
pub fn publish_theme(
    theme: &m5tui_themes::Theme,
    author: &Author,
    version: &str,
    description: &str,
    tags: &[String],
) -> Result<PublishDraft, MarketError> {
    if theme.name.is_empty() {
        return Err(MarketError::Publish("theme name is empty".to_string()));
    }

    let yaml = serialize_theme(theme);
    let size = yaml.len() as u64;
    let hash = Sha256::digest(&yaml);
    let sha256 = hex::encode(hash);
    let id = slugify(&theme.name);

    let entry = ThemeEntry {
        id: id.clone(),
        name: theme.name.clone(),
        version: version.to_string(),
        author: author.clone(),
        description: description.to_string(),
        sha256: sha256.clone(),
        size_bytes: size,
        stars: String::new(),
        installs: 0,
        preview_swatch: preview_swatch(theme),
        tested_omp_versions: Vec::new(),
        tags: tags.to_vec(),
        download_url: format!("https://m5tui.community.market/assets/{id}.yaml"),
    };
    let asset_path = format!("market/assets/{id}.yaml");

    Ok(PublishDraft {
        entry,
        asset_path,
        asset_yaml: yaml,
        sha256,
    })
}

/// Offline cache: write/read the catalog JSON through a `Driver`.
#[derive(Debug, Default, Clone, Copy)]
pub struct OfflineCache;

impl OfflineCache {
    pub fn new() -> Self {
        Self
    }

    /// Persist a catalog to `market/catalog.json` under the driver root.
    pub fn save(
        &self,
        driver: &mut dyn m5tui_persist::Driver,
        catalog: &Catalog,
    ) -> Result<(), MarketError> {
        let text = serialize_catalog(catalog)?;
        driver
            .save(CATALOG_CACHE_PATH, text.as_bytes())
            .map_err(|e| MarketError::Persist(e.to_string()))
    }

    /// Load a catalog from `market/catalog.json` under the driver root.
    pub fn load(&self, driver: &dyn m5tui_persist::Driver) -> Result<Catalog, MarketError> {
        let bytes = driver
            .load(CATALOG_CACHE_PATH)
            .map_err(|e| MarketError::Persist(e.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|e| MarketError::Catalog(e.to_string()))?;
        parse_catalog(&text)
    }
}

/// HTTPS-backed market client using `ureq`.
pub struct UreqMarketClient {
    catalog_url: String,
    catalog: std::sync::Mutex<Option<Catalog>>,
}

impl UreqMarketClient {
    pub fn new(catalog_url: impl Into<String>) -> Self {
        Self {
            catalog_url: catalog_url.into(),
            catalog: std::sync::Mutex::new(None),
        }
    }
}

impl MarketClient for UreqMarketClient {
    fn fetch_catalog(&self) -> Result<Catalog, MarketError> {
        let text = ureq::get(&self.catalog_url)
            .call()
            .map_err(|e| MarketError::Network(e.to_string()))?
            .into_string()
            .map_err(|e| MarketError::Network(e.to_string()))?;
        let catalog = parse_catalog(&text)?;
        let mut guard = match self.catalog.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        *guard = Some(catalog.clone());
        Ok(catalog)
    }

    fn download_theme(&mut self, id: &str) -> Result<String, MarketError> {
        let catalog = {
            let guard = match self.catalog.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            guard
                .as_ref()
                .ok_or_else(|| MarketError::Catalog("fetch catalog first".to_string()))?
                .clone()
        };
        let entry = catalog
            .find(id)
            .ok_or_else(|| MarketError::NotFound(id.to_string()))?;
        let text = ureq::get(&entry.download_url)
            .call()
            .map_err(|e| MarketError::Network(e.to_string()))?
            .into_string()
            .map_err(|e| MarketError::Network(e.to_string()))?;
        Ok(text)
    }

    fn preview_theme(&self, yaml: &str) -> Result<m5tui_themes::Theme, MarketError> {
        preview_theme(yaml)
    }
}

/// Stub client backed by canned catalog data and an in-memory store.
pub struct StubMarketClient {
    catalog: Catalog,
    store: HashMap<String, String>,
}

impl StubMarketClient {
    pub fn with_canned() -> Self {
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let mut store = HashMap::new();
        store.insert("coldwire".to_string(), yaml.to_string());
        Self {
            catalog: Catalog {
                version: 1,
                generated_at: "2026-06-15T22:00:00Z".to_string(),
                entries: vec![ThemeEntry {
                    id: "coldwire".to_string(),
                    name: "Coldwire".to_string(),
                    version: "0.2.0".to_string(),
                    author: Author::new("forest"),
                    description: "Hero theme".to_string(),
                    sha256: String::new(),
                    size_bytes: 0,
                    stars: String::new(),
                    installs: 0,
                    preview_swatch: vec![
                        "#0a1428".to_string(),
                        "#d0e0ff".to_string(),
                        "#00d8ff".to_string(),
                    ],
                    tested_omp_versions: Vec::new(),
                    tags: vec!["cyberpunk".to_string(), "synthwave".to_string()],
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
        preview_theme(yaml)
    }
}

fn preview_swatch(theme: &m5tui_themes::Theme) -> Vec<String> {
    vec![
        format!("#{:06x}", theme.palette.bg.to_rgb24()),
        format!("#{:06x}", theme.palette.fg.to_rgb24()),
        format!("#{:06x}", theme.palette.accent.to_rgb24()),
    ]
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

fn serialize_theme(theme: &m5tui_themes::Theme) -> String {
    use m5tui_themes::schema::{CursorStyle, Density, GlyphSet, ScrollbarStyle};

    let mut out = String::new();
    out.push_str("name: ");
    out.push_str(&theme.name);
    out.push('\n');
    if !theme.author.is_empty() {
        out.push_str("author: ");
        out.push_str(&theme.author);
        out.push('\n');
    }
    out.push_str("brightness: ");
    out.push_str(&theme.brightness.to_string());
    out.push('\n');

    out.push_str("palette:\n");
    let swatches = [
        ("bg", theme.palette.bg),
        ("fg", theme.palette.fg),
        ("accent", theme.palette.accent),
        ("dim", theme.palette.dim),
        ("warn", theme.palette.warn),
        ("err", theme.palette.err),
        ("ok", theme.palette.ok),
        ("sel_bg", theme.palette.sel_bg),
        ("sel_fg", theme.palette.sel_fg),
        ("prompt", theme.palette.prompt),
    ];
    for (name, rgb) in swatches {
        out.push_str("  ");
        out.push_str(name);
        out.push_str(": \"#");
        out.push_str(&format!("{:06x}", rgb.to_rgb24()));
        out.push_str("\"\n");
    }

    out.push_str("glyphs:\n");
    out.push_str("  box: ");
    out.push_str(match theme.glyphs.box_ {
        GlyphSet::Ascii => "ascii",
        GlyphSet::Heavy => "heavy",
        GlyphSet::Double => "double",
        GlyphSet::Rounded => "rounded",
    });
    out.push('\n');
    out.push_str("  scrollbar: ");
    out.push_str(match theme.glyphs.scrollbar {
        ScrollbarStyle::Block => "block",
        ScrollbarStyle::Thin => "thin",
        ScrollbarStyle::Arrow => "arrow",
    });
    out.push('\n');
    out.push_str("  cursor: ");
    out.push_str(match theme.glyphs.cursor {
        CursorStyle::Block => "block",
        CursorStyle::Underline => "underline",
        CursorStyle::Bar => "bar",
    });
    out.push('\n');

    out.push_str("layout:\n");
    out.push_str("  density: ");
    out.push_str(match theme.layout.density {
        Density::Compact => "compact",
        Density::Cozy => "cozy",
        Density::Comfy => "comfy",
    });
    out.push('\n');
    out.push_str("  show_clock: ");
    out.push_str(bool_name(theme.layout.show_clock));
    out.push('\n');
    out.push_str("  show_synthwave: ");
    out.push_str(bool_name(theme.layout.show_synthwave));
    out.push('\n');

    out.push_str("animation:\n");
    out.push_str("  level: ");
    out.push_str(&theme.animation.level.to_string());
    out.push('\n');
    out.push_str("  scanline: ");
    out.push_str(bool_name(theme.animation.scanline));
    out.push('\n');
    out.push_str("  phosphor_decay: ");
    out.push_str(bool_name(theme.animation.phosphor_decay));
    out.push('\n');
    out.push_str("  glitch_on_event: ");
    out.push_str(bool_name(theme.animation.glitch_on_event));
    out.push('\n');

    out.push_str("sound:\n");
    out.push_str("  enabled: ");
    out.push_str(bool_name(theme.sound.enabled));
    out.push('\n');
    if !theme.sound.boot.is_empty() {
        out.push_str("  boot: \"");
        out.push_str(&theme.sound.boot);
        out.push_str("\"\n");
    }
    if !theme.sound.click.is_empty() {
        out.push_str("  click: \"");
        out.push_str(&theme.sound.click);
        out.push_str("\"\n");
    }
    if !theme.sound.arp.is_empty() {
        out.push_str("  arp: \"");
        out.push_str(&theme.sound.arp);
        out.push_str("\"\n");
    }

    out
}

fn bool_name(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use m5tui_persist::{Driver, MemoryDriver};
    fn sample_catalog() -> Catalog {
        Catalog {
            version: 1,
            generated_at: "2026-06-15T22:00:00Z".to_string(),
            entries: vec![ThemeEntry {
                id: "coldwire+".to_string(),
                name: "Coldwire+".to_string(),
                version: "1.2.0".to_string(),
                author: Author::new("forest"),
                description: "Coldwire with extra synthwave.".to_string(),
                sha256: "abcd".to_string(),
                size_bytes: 8192,
                stars: "4.9".to_string(),
                installs: 12345,
                preview_swatch: vec![
                    "#0a1428".to_string(),
                    "#00d8ff".to_string(),
                    "#ff00aa".to_string(),
                ],
                tested_omp_versions: vec!["15.13.3".to_string()],
                tags: vec!["cyberpunk".to_string(), "synthwave".to_string()],
                download_url: "https://example.com/coldwire.yaml".to_string(),
            }],
        }
    }

    #[test]
    fn parse_json_catalog() {
        let text = r##"{
            "version": 1,
            "generated_at": "2026-06-15T22:00:00Z",
            "themes": [
                {
                    "id": "coldwire+",
                    "name": "Coldwire+",
                    "author": "forest",
                    "version": "1.2.0",
                    "sha256": "abcd",
                    "size_bytes": 8192,
                    "stars": "4.9",
                    "installs": 12345,
                    "description": "Coldwire with extra synthwave.",
                    "preview_swatch": ["#0a1428", "#00d8ff", "#ff00aa"],
                    "tested_omp_versions": ["15.13.3"],
                    "tags": ["cyberpunk", "synthwave"],
                    "download_url": "https://example.com/coldwire.yaml"
                }
            ]
        }"##;
        let c = parse_catalog(text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.version, 1);
        assert_eq!(c.entries.len(), 1);
        let e = &c.entries[0];
        assert_eq!(e.id, "coldwire+");
        assert_eq!(e.author.name, "forest");
        assert_eq!(e.author.url, "");
        assert_eq!(e.preview_swatch.len(), 3);
    }

    #[test]
    fn author_deserializes_from_object() {
        let text = r#"{
            "version": 1,
            "themes": [{
                "id": "x",
                "name": "X",
                "version": "1.0.0",
                "author": {"name": "ada", "url": "https://ada.dev"}
            }]
        }"#;
        let c = parse_catalog(text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            c.entries[0].author,
            Author::with_url("ada", "https://ada.dev")
        );
    }

    #[test]
    fn serialize_and_parse_round_trip() {
        let c = sample_catalog();
        let text = serialize_catalog(&c).unwrap_or_else(|e| panic!("{e}"));
        let parsed = parse_catalog(&text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(parsed, c);
    }

    #[test]
    fn install_idempotency() {
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let mut driver = MemoryDriver::new();
        install_theme(&mut driver, "coldwire", yaml).unwrap_or_else(|e| panic!("{e}"));
        install_theme(&mut driver, "coldwire", yaml).unwrap_or_else(|e| panic!("{e}"));
        let files = driver.list(THEMES_DIR).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(files.len(), 1);
        let bytes = driver.load(&files[0]).unwrap_or_else(|e| panic!("{e}"));
        let saved = String::from_utf8(bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(saved, yaml);
    }

    #[test]
    fn preview_render() {
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let theme = preview_theme(yaml).unwrap_or_else(|e| panic!("{e}"));
        let frame = render_preview(&theme);
        assert_eq!(frame.cells.len(), m5tui_core::ROWS);
        let top_left = &frame.cells[0][0];
        assert_eq!(top_left.bg, theme.palette.bg.0);
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
            .download_theme("coldwire")
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

    #[test]
    fn publish_produces_entry_and_asset_path() {
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let theme = preview_theme(yaml).unwrap_or_else(|e| panic!("{e}"));
        let draft = publish_theme(
            &theme,
            &Author::new("forest"),
            "1.2.0",
            "Coldwire with extra synthwave.",
            &["cyberpunk".to_string(), "synthwave".to_string()],
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(draft.entry.id, "coldwire");
        assert_eq!(draft.entry.name, "coldwire");
        assert_eq!(draft.entry.version, "1.2.0");
        assert!(!draft.sha256.is_empty());
        assert!(draft.asset_path.ends_with("/coldwire.yaml"));
        assert!(draft.asset_yaml.contains("name: coldwire"));
        assert_eq!(draft.entry.preview_swatch.len(), 3);
    }

    #[test]
    fn published_yaml_roundtrips_through_parser() {
        let yaml = include_str!("../../../themes/coldwire.yaml");
        let theme = preview_theme(yaml).unwrap_or_else(|e| panic!("{e}"));
        let draft = publish_theme(&theme, &Author::new("forest"), "1.0.0", "desc", &[])
            .unwrap_or_else(|e| panic!("{e}"));
        let reparsed = preview_theme(&draft.asset_yaml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(reparsed.name, theme.name);
    }
}
