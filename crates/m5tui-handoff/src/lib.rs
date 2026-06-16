//! `m5tui-handoff` — handoff viewer and vault search.
//!
//! M6 scope: parse handoff metadata, render a tiny subset of Markdown
//! to TUI lines, and define a vault search trait. Real SFTP fetch and
//! obsidian-memory invocation are deferred.

/// A handoff document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    pub project: String,
    pub agent_prompt: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub updated: u64,
}

impl Handoff {
    /// Build a one-line display string for a picker.
    pub fn display(&self) -> String {
        let tags = if self.tags.is_empty() {
            "no-tags".to_string()
        } else {
            self.tags.join(",")
        };
        format!("{} [{}] {}", self.project, tags, self.summary)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffError {
    NotFound(String),
    Parse(String),
    Vault(String),
}

impl std::fmt::Display for HandoffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "not found: {s}"),
            Self::Parse(s) => write!(f, "parse: {s}"),
            Self::Vault(s) => write!(f, "vault: {s}"),
        }
    }
}

impl std::error::Error for HandoffError {}

/// Storage for handoff documents.
pub trait HandoffStore: Send + Sync {
    fn list(&self) -> Vec<&Handoff>;
    fn search(&self, query: &str) -> Vec<&Handoff>;
    fn get(&self, project: &str) -> Option<&Handoff>;
}

/// In-memory store with sample handoffs.
#[derive(Debug, Default, Clone)]
pub struct InMemoryStore {
    handoffs: Vec<Handoff>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_samples() -> Self {
        let mut s = Self::new();
        s.handoffs.push(Handoff {
            project: "m5Tui".to_string(),
            agent_prompt: "You are working on the m5Tui TUI.".to_string(),
            summary: "M2 theme engine shipped.".to_string(),
            tags: vec!["rust".to_string(), "tui".to_string()],
            updated: 1718500000,
        });
        s.handoffs.push(Handoff {
            project: "advdeck-bridge".to_string(),
            agent_prompt: "You are working on advdeck-bridge.".to_string(),
            summary: "Bridge voice memo queue.".to_string(),
            tags: vec!["voice".to_string()],
            updated: 1718400000,
        });
        s
    }
}

impl HandoffStore for InMemoryStore {
    fn list(&self) -> Vec<&Handoff> {
        self.handoffs.iter().collect()
    }

    fn search(&self, query: &str) -> Vec<&Handoff> {
        let q = query.to_lowercase();
        self.handoffs
            .iter()
            .filter(|h| {
                h.project.to_lowercase().contains(&q)
                    || h.summary.to_lowercase().contains(&q)
                    || h.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    fn get(&self, project: &str) -> Option<&Handoff> {
        self.handoffs.iter().find(|h| h.project == project)
    }
}

/// A rendered line with a style tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineStyle {
    Normal,
    Heading,
    Bold,
    Italic,
    Code,
    Bullet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub style: LineStyle,
    pub text: String,
}

/// Minimal Markdown renderer for inline TUI.
pub fn render_markdown(src: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for raw in src.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            out.push(Line {
                style: LineStyle::Heading,
                text: rest.to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("## ") {
            out.push(Line {
                style: LineStyle::Heading,
                text: rest.to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("- ") {
            out.push(Line {
                style: LineStyle::Bullet,
                text: format!("• {rest}"),
            });
        } else {
            out.push(Line {
                style: LineStyle::Normal,
                text: line.to_string(),
            });
        }
    }
    out
}

/// A vault search hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub title: String,
    pub snippet: String,
}

/// Vault search abstraction.
pub trait VaultSearch: Send + Sync {
    fn query(&self, q: &str) -> Result<Vec<Hit>, HandoffError>;
}

/// Stub vault returning canned hits.
pub struct StubVaultClient;

impl Default for StubVaultClient {
    fn default() -> Self {
        Self
    }
}

impl StubVaultClient {
    pub fn new() -> Self {
        Self
    }
}

impl VaultSearch for StubVaultClient {
    fn query(&self, _q: &str) -> Result<Vec<Hit>, HandoffError> {
        Ok(vec![
            Hit {
                path: "vault/m5tui.md".to_string(),
                title: "m5Tui".to_string(),
                snippet: "M2 shipped the theme engine.".to_string(),
            },
            Hit {
                path: "vault/advdeck.md".to_string(),
                title: "advdeck-bridge".to_string(),
                snippet: "Voice memo queue design.".to_string(),
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_heading_and_bullet() {
        let src = "# Title
- item one
plain line";
        let lines = render_markdown(src);
        assert_eq!(lines[0].style, LineStyle::Heading);
        assert_eq!(lines[0].text, "Title");
        assert_eq!(lines[1].style, LineStyle::Bullet);
        assert!(lines[1].text.contains("item one"));
        assert_eq!(lines[2].style, LineStyle::Normal);
    }

    #[test]
    fn handoff_store_search_by_tag() {
        let store = InMemoryStore::with_samples();
        let hits = store.search("rust");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].project, "m5Tui");
    }

    #[test]
    fn handoff_store_get_missing() {
        let store = InMemoryStore::with_samples();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn handoff_display_includes_tags() {
        let h = Handoff {
            project: "x".to_string(),
            agent_prompt: "p".to_string(),
            summary: "s".to_string(),
            tags: vec!["a".to_string(), "b".to_string()],
            updated: 0,
        };
        assert!(h.display().contains("[a,b]"));
    }

    #[test]
    fn stub_vault_returns_hits() {
        let v = StubVaultClient::new();
        let hits = v.query("anything").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(hits.len(), 2);
    }
}
