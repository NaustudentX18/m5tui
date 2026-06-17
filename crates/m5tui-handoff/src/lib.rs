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
    // Legacy variants retained for backwards compatibility with code
    // written against the initial M6 surface.
    Normal,
    Heading,
    Bold,
    Italic,
    Code,
    Bullet,
    // New variants added in wave 12 for the CommonMark-subset renderer.
    /// Plain prose line.
    Body,
    /// Inline code span on a prose line (no surrounding fence).
    InlineCode,
    /// `1. ` ordered list item.
    Numbered,
    /// `> ` block quote line.
    Quote,
    /// `---` horizontal rule.
    Rule,
    /// Blank separator line.
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub style: LineStyle,
    pub text: String,
}

/// Width of the inline TUI framebuffer. The renderer wraps every line
/// so it fits within this budget; list and quote continuations are
/// indented to match their marker width.
pub const FRAME_COLS: usize = 40;

/// Render a CommonMark subset to TUI lines.
///
/// Supported elements:
///   * ATX headings `#`–`######`
///   * `**bold**` and `*italic*` inline emphasis (rendered with marker glyphs)
///   * `` `inline code` `` (inline spans; fences are also recognised)
///   * Fenced code blocks ``` ``` ```
///   * Unordered list items `- ` or `* `
///   * Ordered list items `1. `, `2. `, ...
///   * Block quotes `> `
///   * Links `[text](url)` rendered as `text (url)`
///   * Horizontal rules `---`
///   * Blank lines
///   * Plain prose
///
/// Lines wider than [`FRAME_COLS`] are word-wrapped on spaces. A single
/// word longer than the wrap point is broken at column 38 with a `-`
/// suffix and continued on the next line at indent 0. List, ordered,
/// and quote continuations are indented to match their marker width.
pub fn render_markdown(src: &str) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut lines = src.lines().peekable();

    while let Some(raw) = lines.next() {
        let line = raw.trim_end_matches('\r');

        // Blank line.
        if line.trim().is_empty() {
            out.push(Line {
                style: LineStyle::Blank,
                text: String::new(),
            });
            continue;
        }

        // Fenced code block: ``` ... ```
        if line.trim_start().starts_with("```") {
            let mut code_lines: Vec<String> = Vec::new();
            for inner in lines.by_ref() {
                let inner = inner.trim_end_matches('\r');
                if inner.trim_start().starts_with("```") {
                    break;
                }
                code_lines.push(inner.to_string());
            }
            for cl in code_lines {
                out.push(Line {
                    style: LineStyle::Code,
                    text: cl,
                });
            }
            continue;
        }

        // Horizontal rule: --- (or more dashes) on a line by itself.
        if is_horizontal_rule(line) {
            out.push(Line {
                style: LineStyle::Rule,
                text: String::new(),
            });
            continue;
        }

        // ATX heading: 1-6 leading `#` characters followed by a space.
        if let Some(rest) = strip_heading(line) {
            let text = apply_inline(rest);
            push_wrapped(&mut out, LineStyle::Heading, &text, 0);
            continue;
        }

        // Block quote: leading `> ` (or `>` alone).
        if let Some(rest) = strip_quote(line) {
            let text = apply_inline(rest);
            push_wrapped(&mut out, LineStyle::Quote, &text, 2);
            continue;
        }

        // Ordered list item: `<digits>. ` prefix.
        if let Some((marker, rest)) = strip_ordered(line) {
            let indent = marker.len();
            let text = apply_inline(rest);
            let first = format!("{}{}", marker, text);
            push_wrapped_with_marker(&mut out, LineStyle::Numbered, &first, indent);
            continue;
        }

        // Unordered list item: `- ` or `* `.
        // We render the marker as a literal `• ` so the framebuffer shows a
        // recognisable bullet. Continuation rows are indented to match.
        if let Some((_, rest)) = strip_unordered(line) {
            let indent = 2usize;
            let text = apply_inline(rest);
            let first = format!("\u{2022} {text}");
            push_wrapped_with_marker(&mut out, LineStyle::Bullet, &first, indent);
            continue;
        }

        // Plain prose (or a link line). We emit `Normal` (the legacy
        // body variant) so the existing renderer tests keep passing; the
        // new `Body` variant is reserved for callers that want semantic
        // naming.
        let text = apply_inline(line);
        push_wrapped(&mut out, LineStyle::Normal, &text, 0);
    }

    out
}

/// True when `line` is a horizontal rule: three or more `-`, `_`, or
/// `*` characters separated only by optional spaces.
fn is_horizontal_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let ch = t.chars().next().expect("non-empty");
    if ch != '-' && ch != '_' && ch != '*' {
        return false;
    }
    t.chars().all(|c| c == ch || c == ' ')
}

/// Strip a leading `#`–`######` heading marker. Returns the rest after
/// the first space, or `None` if the line is not a heading.
fn strip_heading(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'#' && i < 6 {
        i += 1;
    }
    if i == 0 || i > 6 {
        return None;
    }
    if bytes.get(i) != Some(&b' ') {
        return None;
    }
    Some(&line[i + 1..])
}

/// Strip a leading `> ` (or `>` followed by whitespace) block-quote
/// marker.
fn strip_quote(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let _leading_ws = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix('>')?;
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    Some(rest)
}

/// Strip an ordered-list marker like `1. ` and return `(marker, rest)`.
fn strip_ordered(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    if bytes.get(i) != Some(&b'.') {
        return None;
    }
    if bytes.get(i + 1) != Some(&b' ') {
        return None;
    }
    Some((&line[..i + 2], &line[i + 2..]))
}

/// Strip an unordered-list marker (`- ` or `* `) and return
/// `(marker, rest)`.
fn strip_unordered(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let first = *bytes.first()?;
    if first != b'-' && first != b'*' {
        return None;
    }
    if bytes.get(1) != Some(&b' ') {
        return None;
    }
    Some((&line[..2], &line[2..]))
}

/// Apply inline transforms to a single prose line:
///   * `**foo**` -> `*foo*`  (rendered bold-ish on a 40-col framebuffer)
///   * `*foo*`   -> `_foo_`  (italic-ish)
///   * `` `foo` `` -> `foo` (marker dropped; style tag carries the meaning)
///   * `[text](url)` -> `text (url)`
///
/// The function only mutates ASCII delimiter runs so it cannot
/// accidentally eat a Unicode apostrophe.
fn apply_inline(src: &str) -> String {
    // Links first so emphasis inside `[text]` is not double-processed
    // by the bold/italic pass (which would still work but is wasteful).
    let mut s = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(end) = find_link_close(bytes, i) {
                let text = &src[i + 1..end];
                // After `]` expect `(url)`.
                let after = end + 1;
                if bytes.get(after) == Some(&b'(') {
                    if let Some(close_paren) = src[after + 1..].find(')') {
                        let url = &src[after + 1..after + 1 + close_paren];
                        s.push_str(text);
                        s.push_str(" (");
                        s.push_str(url);
                        s.push(')');
                        i = after + 1 + close_paren + 1;
                        continue;
                    }
                }
                s.push('[');
                i += 1;
                continue;
            } else {
                s.push('[');
                i += 1;
                continue;
            }
        }
        // Inline code span: `foo` -> foo
        if bytes[i] == b'`' {
            if let Some(close) = src[i + 1..].find('`') {
                s.push_str(&src[i + 1..i + 1 + close]);
                i = i + 1 + close + 1;
                continue;
            } else {
                s.push('`');
                i += 1;
                continue;
            }
        }
        // Bold: **foo** -> *foo*
        if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'*') {
            if let Some(close) = find_double_close(bytes, i + 2) {
                s.push('*');
                s.push_str(&src[i + 2..close]);
                s.push('*');
                i = close + 2;
                continue;
            }
        }
        // Italic: *foo* -> _foo_
        if bytes[i] == b'*' {
            if let Some(close) = src[i + 1..].find('*') {
                // Don't eat a stray `*` that's actually the end of a bold.
                if close > 0 {
                    s.push('_');
                    s.push_str(&src[i + 1..i + 1 + close]);
                    s.push('_');
                    i = i + 1 + close + 1;
                    continue;
                }
            }
        }
        // Push one char (UTF-8 safe).
        let ch = src[i..].chars().next().expect("index at char boundary");
        s.push(ch);
        i += ch.len_utf8();
    }
    s
}

/// Find the `]` that closes a link opened at `start` (which must point
/// at `[`). Returns the byte index of the `]` or `None`.
fn find_link_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b']' {
            return Some(i);
        }
        // A newline would end the line; bail out.
        if bytes[i] == b'\n' {
            return None;
        }
        i += 1;
    }
    None
}

/// Find the closing `**` after `start` for a bold run.
fn find_double_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Push a line and any wrapped continuations. `indent` is the number
/// of leading spaces used for continuation rows.
fn push_wrapped(out: &mut Vec<Line>, style: LineStyle, text: &str, indent: usize) {
    let wrap = FRAME_COLS.saturating_sub(indent).max(1);
    let mut first = true;
    for chunk in wrap_text(text, wrap) {
        if first {
            out.push(Line {
                style: style.clone(),
                text: chunk,
            });
            first = false;
        } else {
            out.push(Line {
                style: style.clone(),
                text: format!("{}{}", " ".repeat(indent), chunk),
            });
        }
    }
}

/// Like [`push_wrapped`] but the first row already includes a marker
/// (e.g. `- `, `1. `, `> `); only continuation rows are indented.
fn push_wrapped_with_marker(
    out: &mut Vec<Line>,
    style: LineStyle,
    first_text: &str,
    indent: usize,
) {
    let wrap = FRAME_COLS.saturating_sub(indent).max(1);
    let mut iter = wrap_text(first_text, wrap).into_iter();
    if let Some(head) = iter.next() {
        out.push(Line {
            style: style.clone(),
            text: head,
        });
    }
    for chunk in iter {
        out.push(Line {
            style: style.clone(),
            text: format!("{}{}", " ".repeat(indent), chunk),
        });
    }
}

/// Word-wrap `text` to at most `wrap` columns per line. Splits on
/// single ASCII spaces. A single word longer than `wrap` is broken at
/// column `wrap - 2` with a `-` suffix and resumed on the next row.
fn wrap_text(text: &str, wrap: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if text.is_empty() {
        out.push(String::new());
        return out;
    }
    let mut current = String::new();
    for word in text.split(' ') {
        // Collapse runs of spaces (split(' ') yields empties for them).
        if word.is_empty() {
            continue;
        }
        // Long-word break: append the chunk that fits, possibly ending with
        // a `-`, then push and reset. Repeats until the word is consumed.
        let mut remaining = word;
        loop {
            let room = wrap.saturating_sub(current.len());
            if remaining.len() <= room {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(remaining);
                break;
            }
            // Word doesn't fit on the current line. If current is empty we
            // must hard-break the word itself.
            if current.is_empty() {
                // Reserve 1 column for the `-` continuation mark.
                let take = wrap.saturating_sub(2).max(1);
                if remaining.len() <= take {
                    out.push(remaining.to_string());
                } else {
                    out.push(format!("{}-", &remaining[..take]));
                }
                remaining = &remaining[take..];
            } else {
                // Flush the current line, then retry the word on a fresh row.
                out.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
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

impl Hit {
    /// Format a single hit as one line for a 40-column TUI display.
    /// The path is truncated to 24 chars, then the title follows.
    pub fn display_line(&self) -> String {
        let path = if self.path.len() > 24 {
            format!("~{}", &self.path[self.path.len() - 23..])
        } else {
            self.path.clone()
        };
        format!("{:<24} {}", path, self.title)
    }
}

/// Parse one JSONL line into a `Hit`. The line is expected to be
/// `{"path":"...","title":"...","snippet":"..."}`. Returns a
/// `HandoffError::Parse` on any malformed input.
pub fn parse_hit_jsonl(line: &str) -> Result<Hit, HandoffError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(HandoffError::Parse("empty line".to_string()));
    }
    let v: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| HandoffError::Parse(format!("json: {e}")))?;
    let path = v
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| HandoffError::Parse("missing path".to_string()))?
        .to_string();
    let title = v
        .get("title")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    let snippet = v
        .get("snippet")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    Ok(Hit {
        path,
        title,
        snippet,
    })
}

/// Parse a stream of JSONL hits into a `Vec<Hit>`. Stops on the first
/// parse error and returns it; blank lines are ignored.
pub fn parse_hits_jsonl(stream: &str) -> Result<Vec<Hit>, HandoffError> {
    let mut out = Vec::new();
    for line in stream.lines() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(parse_hit_jsonl(line)?);
    }
    Ok(out)
}

/// A single search-history entry: the query and the time it was run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub query: String,
    pub ran_at: u64,
}

/// In-memory append-only history of vault searches.
#[derive(Debug, Default, Clone)]
pub struct SearchHistory {
    entries: Vec<HistoryEntry>,
}

impl SearchHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new search query. Empty queries are ignored.
    pub fn record(&mut self, query: &str, ran_at: u64) {
        if query.is_empty() {
            return;
        }
        self.entries.push(HistoryEntry {
            query: query.to_string(),
            ran_at,
        });
    }

    /// Number of recorded searches.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no searches have been recorded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Most-recent searches first, capped at `n` entries.
    pub fn recent(&self, n: usize) -> Vec<&HistoryEntry> {
        let skip = self.entries.len().saturating_sub(n);
        self.entries[skip..].iter().rev().collect()
    }

    /// Serialize the history as one JSONL line per entry.
    pub fn to_jsonl(&self) -> String {
        self.entries
            .iter()
            .map(|e| serde_json::json!({"query": e.query, "ran_at": e.ran_at}).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Re-hydrate the history from JSONL. Malformed lines are silently
    /// dropped so a corrupt history file doesn't break the picker.
    pub fn load_jsonl(&mut self, stream: &str) {
        for line in stream.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let q = v.get("query").and_then(|q| q.as_str()).unwrap_or("");
                let t = v.get("ran_at").and_then(|t| t.as_u64()).unwrap_or(0);
                if !q.is_empty() {
                    self.entries.push(HistoryEntry {
                        query: q.to_string(),
                        ran_at: t,
                    });
                }
            }
        }
    }
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

/// Vault client that serves a pre-fetched corpus without any I/O.
///
/// Useful for tests, the `;m` memory search, and any context where SSH
/// is unavailable. The corpus is cloned on each [`VaultSearch::query`]
/// call so the client can be reused across many searches.
#[derive(Debug, Default, Clone)]
pub struct InMemoryVaultClient {
    corpus: Vec<Hit>,
}

impl InMemoryVaultClient {
    /// Build a client from an already-parsed corpus.
    pub fn new(hits: Vec<Hit>) -> Self {
        Self { corpus: hits }
    }

    /// Parse a JSONL stream (one [`Hit`] per non-blank line) into a
    /// client. Returns the same [`HandoffError::Parse`] as
    /// [`parse_hits_jsonl`] on the first malformed row.
    pub fn from_jsonl(stream: &str) -> Result<Self, HandoffError> {
        let hits = parse_hits_jsonl(stream)?;
        Ok(Self { corpus: hits })
    }

    /// Number of hits currently held.
    pub fn len(&self) -> usize {
        self.corpus.len()
    }

    /// True when the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.corpus.is_empty()
    }
}

impl VaultSearch for InMemoryVaultClient {
    /// Returns a clone of the pre-fetched corpus regardless of the query.
    /// The trait signature accepts `q` so the in-memory client is a
    /// drop-in for future filtered clients (rank, BM25, etc.).
    fn query(&self, _q: &str) -> Result<Vec<Hit>, HandoffError> {
        Ok(self.corpus.clone())
    }
}

/// Build the system-prompt prefix for a `;continue` command. The
/// resulting string is prepended to a fresh OMP session so the agent
/// inherits the project's working context.
pub fn continue_prompt(handoff: &Handoff) -> String {
    let tags = if handoff.tags.is_empty() {
        "no-tags".to_string()
    } else {
        handoff.tags.join(", ")
    };
    format!(
        "Continuing project `{project}` (tags: {tags}).\n\nLast summary: {summary}\n\nWorking prompt:\n{prompt}\n",
        project = handoff.project,
        summary = handoff.summary,
        prompt = handoff.agent_prompt,
    )
}

/// Build the OMP `session.create` frame for a `;continue` command. The
/// `id` is used by the orchestrator to correlate the reply; the
/// `model` is the LLM identifier (e.g. `qwen3-14b`).
pub fn continue_frame(
    handoff: &Handoff,
    id: impl Into<String>,
    model: impl Into<String>,
) -> m5tui_omp::OmpFrame {
    let mut args = std::collections::HashMap::new();
    args.insert("model".to_string(), model.into());
    args.insert("system_prefix".to_string(), continue_prompt(handoff));
    args.insert("project".to_string(), handoff.project.clone());
    m5tui_omp::OmpFrame::ToolCall {
        id: id.into(),
        tool: "session.create".to_string(),
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_heading_and_bullet() {
        let src = "# Title\n- item one\nplain line";
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

    #[test]
    fn continue_prompt_includes_project_and_summary() {
        let h = InMemoryStore::with_samples()
            .get("m5Tui")
            .cloned()
            .unwrap_or_else(|| panic!("sample missing"));
        let prompt = continue_prompt(&h);
        assert!(prompt.contains("m5Tui"));
        assert!(prompt.contains("M2 theme engine shipped."));
        assert!(prompt.contains("rust"));
    }

    #[test]
    fn continue_frame_is_session_create() {
        let h = InMemoryStore::with_samples()
            .get("advdeck-bridge")
            .cloned()
            .unwrap_or_else(|| panic!("sample missing"));
        let f = continue_frame(&h, "c1", "qwen3-14b");
        match f {
            m5tui_omp::OmpFrame::ToolCall { id, tool, args } => {
                assert_eq!(id, "c1");
                assert_eq!(tool, "session.create");
                assert_eq!(args.get("model").map(String::as_str), Some("qwen3-14b"));
                assert_eq!(
                    args.get("project").map(String::as_str),
                    Some("advdeck-bridge")
                );
                assert!(args.contains_key("system_prefix"));
            }
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[test]
    fn parse_single_jsonl_hit() {
        let line = r#"{"path":"vault/m5tui.md","title":"m5Tui","snippet":"M2 done"}"#;
        let h = parse_hit_jsonl(line).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(h.path, "vault/m5tui.md");
        assert_eq!(h.title, "m5Tui");
        assert_eq!(h.snippet, "M2 done");
    }

    #[test]
    fn parse_hits_jsonl_ignores_blank_lines() {
        let stream = r#"{"path":"a","title":"A","snippet":"a"}
{"path":"b","title":"B","snippet":"b"}
"#;
        let hits = parse_hits_jsonl(stream).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].path, "a");
        assert_eq!(hits[1].path, "b");
    }

    #[test]
    fn parse_hits_jsonl_errors_on_bad_json() {
        let stream = r#"{"path":"a"}
not json"#;
        let r = parse_hits_jsonl(stream);
        assert!(r.is_err());
    }

    #[test]
    fn hit_display_line_fits_40_cols() {
        let h = Hit {
            path: "vault/very/long/path/to/some/file.md".to_string(),
            title: "X".to_string(),
            snippet: "y".to_string(),
        };
        let line = h.display_line();
        assert!(
            line.len() <= 40,
            "line too long: '{line}' len={}",
            line.len()
        );
    }

    #[test]
    fn search_history_records_and_recent() {
        let mut h = SearchHistory::new();
        h.record("rust", 1);
        h.record("tui", 2);
        h.record("m5tui", 3);
        let r = h.recent(2);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].query, "m5tui");
        assert_eq!(r[1].query, "tui");
    }

    #[test]
    fn search_history_ignores_empty() {
        let mut h = SearchHistory::new();
        h.record("", 1);
        h.record("ok", 2);
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn search_history_round_trip() {
        let mut h = SearchHistory::new();
        h.record("alpha", 100);
        h.record("beta", 200);
        let jsonl = h.to_jsonl();
        let mut h2 = SearchHistory::new();
        h2.load_jsonl(&jsonl);
        assert_eq!(h2.len(), 2);
        assert_eq!(h2.recent(1)[0].query, "beta");
    }

    #[test]
    fn search_history_load_silently_drops_bad_lines() {
        let mut h = SearchHistory::new();
        h.load_jsonl(
            r#"not json
{"query":"x","ran_at":1}
{"query":42}
"#,
        );
        assert_eq!(h.len(), 1);
        assert_eq!(h.recent(1)[0].query, "x");
    }
}
