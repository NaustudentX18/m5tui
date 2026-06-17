//! `m5tui-book` — Book of Commands.
//!
//! Spells are data, not code. Each spell has a key, prefix, parameter
//! prompts, a template, and an optional undo template. Templates expand
//! `{param}` placeholders. M5a scope is the registry + expander + undo
//! stack; real on-device author mode is deferred.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Param {
    pub name: String,
    pub prompt: String,
    pub default: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Spell {
    pub name: String,
    pub key: String,
    pub prefix: String,
    pub help: String,
    pub params: Vec<Param>,
    pub template: String,
    pub undo_template: Option<String>,
}

impl Spell {
    /// Validate structural rules: name/key non-empty, template non-empty.
    pub fn validate(&self) -> Result<(), BookError> {
        if self.name.is_empty() {
            return Err(BookError::Invalid("spell name is empty".to_string()));
        }
        if self.key.is_empty() {
            return Err(BookError::Invalid("spell key is empty".to_string()));
        }
        if self.template.is_empty() {
            return Err(BookError::Invalid("spell template is empty".to_string()));
        }
        Ok(())
    }

    /// Expand `{name}` placeholders with provided values. Missing
    /// placeholders are left unchanged.
    pub fn expand(&self, values: &HashMap<String, String>) -> String {
        let mut out = self.template.clone();
        for (k, v) in values {
            out = out.replace(&format!("{{{k}}}"), v);
        }
        out
    }

    pub fn expand_undo(&self, values: &HashMap<String, String>) -> Option<String> {
        self.undo_template.as_ref().map(|t| {
            let mut out = t.clone();
            for (k, v) in values {
                out = out.replace(&format!("{{{k}}}"), v);
            }
            out
        })
    }

    /// Render the prompts the author-mode UI shows for the spell, in
    /// declaration order. Each prompt is one line. The author types the
    /// values back into a list; the order in `out` matches the order
    /// in `self.params`.
    pub fn render_prompts(&self) -> Vec<String> {
        self.params
            .iter()
            .map(|p| {
                let req = if p.required { " *" } else { "" };
                let def = match &p.default {
                    Some(d) => format!(" (default: {d})"),
                    None => String::new(),
                };
                format!("? {}{}{}  ", p.name, req, def) + &p.prompt
            })
            .collect()
    }

    /// Validate that every required parameter has a value in `values`.
    /// Returns the first missing key for a clean error message.
    pub fn validate_values(&self, values: &HashMap<String, String>) -> Result<(), BookError> {
        for p in &self.params {
            if p.required {
                match values.get(&p.name) {
                    Some(v) if !v.is_empty() => {}
                    _ => return Err(BookError::MissingParam(p.name.clone())),
                }
            }
        }
        Ok(())
    }

    /// Construct a values map from defaults and the user-supplied
    /// overrides. The result always has an entry for every parameter
    /// (so the author doesn't have to type them all if defaults are
    /// good enough).
    pub fn values_with_defaults(
        &self,
        overrides: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for p in &self.params {
            let v = overrides
                .get(&p.name)
                .cloned()
                .or_else(|| p.default.clone())
                .unwrap_or_default();
            out.insert(p.name.clone(), v);
        }
        out
    }
}

/// Author-mode editor. Lets the framework create, edit, and validate
/// spells in-process. The on-device UI calls `set_param` to capture
/// the user's typed values, then `commit()` to build a finished Spell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorEditor {
    spell: Spell,
    values: HashMap<String, String>,
}

impl AuthorEditor {
    pub fn new(name: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            spell: Spell {
                name: name.into(),
                key: key.into(),
                ..Spell::default()
            },
            values: HashMap::new(),
        }
    }

    pub fn from_spell(spell: Spell) -> Self {
        Self {
            spell,
            values: HashMap::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.spell.help = help.into();
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.spell.prefix = prefix.into();
        self
    }

    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.spell.template = template.into();
        self
    }

    pub fn with_undo_template(mut self, template: impl Into<String>) -> Self {
        self.spell.undo_template = Some(template.into());
        self
    }

    pub fn add_param(
        mut self,
        name: impl Into<String>,
        prompt: impl Into<String>,
        required: bool,
    ) -> Self {
        self.spell.params.push(Param {
            name: name.into(),
            prompt: prompt.into(),
            required,
            ..Param::default()
        });
        self
    }

    /// Record a value typed by the author. Empty strings are stored as
    /// empty values; the user can override defaults later.
    pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values.insert(name.into(), value.into());
    }

    /// Build a `Spell` with the captured state. The template is left
    /// untouched; the author is expected to set it explicitly via
    /// `with_template`. Validation happens here.
    pub fn commit(self) -> Result<Spell, BookError> {
        self.spell.validate()?;
        Ok(self.spell)
    }

    /// Render the prompt string for a single parameter. Returns None
    /// if the parameter name is not part of this spell.
    pub fn prompt_for(&self, name: &str) -> Option<String> {
        self.spell
            .params
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.prompt.clone())
    }

    /// Read-only access to the current value of a parameter.
    pub fn value_of(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookError {
    Invalid(String),
    NotFound(String),
    Duplicate(String),
    DuplicateName(String),
    MissingParam(String),
    Io(String),
    Parse {
        path: PathBuf,
        line: usize,
        message: String,
    },
}

impl std::fmt::Display for BookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "invalid: {s}"),
            Self::NotFound(s) => write!(f, "not found: {s}"),
            Self::Duplicate(s) => write!(f, "duplicate: {s}"),
            Self::DuplicateName(s) => write!(f, "duplicate name: {s}"),
            Self::MissingParam(s) => write!(f, "missing param: {s}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Parse {
                path,
                line,
                message,
            } => {
                write!(
                    f,
                    "parse error in {} at line {line}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for BookError {}

impl BookError {
    /// Format an `std::io::Error` together with the offending path so
    /// the variant stays cloneable / comparable (mirrors the
    /// `m5tui-persist::PersistError::io` pattern).
    pub fn io(action: &str, path: &Path, e: std::io::Error) -> Self {
        Self::Io(format!("{action} {}: {e}", path.display()))
    }
}

/// Spell storage.
pub trait BookRegistry: Send + Sync {
    fn list(&self) -> Vec<&Spell>;
    fn get(&self, name: &str) -> Option<&Spell>;
    fn add(&mut self, spell: Spell) -> Result<(), BookError>;
    fn remove(&mut self, name: &str) -> Result<(), BookError>;
    fn export_yaml(&self, name: &str) -> Result<String, BookError>;
}

/// In-memory registry.
#[derive(Debug, Default, Clone)]
pub struct InMemoryRegistry {
    spells: HashMap<String, Spell>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a validated spell. Used by directory loaders. Differs
    /// from `BookRegistry::add` only in that it returns the explicit
    /// `DuplicateName` variant instead of the generic `Duplicate`.
    pub fn insert(&mut self, spell: Spell) -> Result<(), BookError> {
        spell.validate()?;
        if self.spells.contains_key(&spell.name) {
            return Err(BookError::DuplicateName(spell.name));
        }
        self.spells.insert(spell.name.clone(), spell);
        Ok(())
    }
}

impl BookRegistry for InMemoryRegistry {
    fn list(&self) -> Vec<&Spell> {
        let mut out: Vec<&Spell> = self.spells.values().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    fn get(&self, name: &str) -> Option<&Spell> {
        self.spells.get(name)
    }

    fn add(&mut self, spell: Spell) -> Result<(), BookError> {
        spell.validate()?;
        if self.spells.contains_key(&spell.name) {
            return Err(BookError::Duplicate(spell.name));
        }
        self.spells.insert(spell.name.clone(), spell);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), BookError> {
        if self.spells.remove(name).is_none() {
            return Err(BookError::NotFound(name.to_string()));
        }
        Ok(())
    }

    fn export_yaml(&self, name: &str) -> Result<String, BookError> {
        let s = self
            .spells
            .get(name)
            .ok_or_else(|| BookError::NotFound(name.to_string()))?;
        let mut out = format!(
            "name: {}
",
            s.name
        );
        out.push_str(&format!(
            "key: {}
",
            s.key
        ));
        out.push_str(&format!(
            "prefix: {}
",
            s.prefix
        ));
        out.push_str(&format!(
            "help: {}
",
            s.help
        ));
        out.push_str(
            "template: |
",
        );
        for line in s.template.lines() {
            out.push_str(&format!(
                "  {line}
"
            ));
        }
        if let Some(u) = &s.undo_template {
            out.push_str(
                "undo_template: |
",
            );
            for line in u.lines() {
                out.push_str(&format!(
                    "  {line}
"
                ));
            }
        }
        Ok(out)
    }
}

/// Undo stack stores expanded commands in reverse order of execution.
#[derive(Debug, Default, Clone)]
pub struct UndoStack {
    entries: Vec<String>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, undo_command: String) {
        self.entries.push(undo_command);
    }

    pub fn pop(&mut self) -> Option<String> {
        self.entries.pop()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Load all `.yaml` files directly under `dir` into a fresh registry.
/// Empty directories are not an error. The walker is non-recursive
/// (M5a only ships a flat spell directory). Files that fail to
/// parse bubble up with the offending path and line; readers see the
/// first failure, not a silently-dropped file.
pub fn load_book_dir(dir: &Path) -> Result<InMemoryRegistry, BookError> {
    let mut registry = InMemoryRegistry::new();
    let entries = std::fs::read_dir(dir).map_err(|e| BookError::io("read_dir", dir, e))?;
    let mut yaml_paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    yaml_paths.sort();
    for path in yaml_paths {
        let body = std::fs::read_to_string(&path)
            .map_err(|e| BookError::io("read_to_string", &path, e))?;
        let spell = parse_spell_yaml(&body).map_err(|message| BookError::Parse {
            path: path.clone(),
            line: 0,
            message,
        })?;
        registry.insert(spell)?;
    }
    Ok(registry)
}

/// Hand-rolled parser for the spell YAML shape used in `book/*.yaml`.
/// Returns a human-readable error message tagged with `line` 0 here
/// (line numbers for inner errors are not threaded through; the
/// surrounding `BookError::Parse` carries the file path).
///
/// Supported top-level keys: `name`, `key`, `prefix`, `help`,
/// `params` (list of objects with `name`/`prompt`/`default`/
/// `required`), `template` (quoted scalar or `|` block scalar),
/// `undo_template` (same shape as `template`, optional).
fn parse_spell_yaml(body: &str) -> Result<Spell, String> {
    let mut spell = Spell::default();
    let mut i = 0usize;
    let lines: Vec<&str> = body.lines().collect();
    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }
        let (key, value) = match line.split_once(':') {
            Some(kv) => kv,
            None => return Err(format!("expected `key: value`, got `{line}`")),
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "name" => spell.name = unquote(value).into_owned(),
            "key" => spell.key = unquote(value).into_owned(),
            "prefix" => spell.prefix = unquote(value).into_owned(),
            "help" => spell.help = unquote(value).into_owned(),
            "params" => {
                if !value.is_empty() {
                    return Err(format!("`params` must be a list, got `{value}`"));
                }
                i += 1;
                while i < lines.len() {
                    let pline = lines[i].trim_end();
                    if pline.is_empty() || pline.starts_with('#') {
                        i += 1;
                        continue;
                    }
                    let stripped = pline.trim_start();
                    if !stripped.starts_with("- ") {
                        break;
                    }
                    // Collect the bullet header plus any continuation
                    // lines that are indented deeper than the bullet
                    // (the param record is a multi-line object).
                    let mut buf = stripped.trim_start_matches("- ").to_string();
                    i += 1;
                    while i < lines.len() {
                        let cont = lines[i];
                        if cont.is_empty() {
                            break;
                        }
                        if cont.starts_with(' ') || cont.starts_with('\t') {
                            buf.push(' ');
                            buf.push_str(cont.trim());
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    spell.params.push(
                        parse_param_item(&buf)
                            .map_err(|m| format!("param #{}: {m}", spell.params.len() + 1))?,
                    );
                }
                continue;
            }
            "template" | "undo_template" => {
                let block = if value == "|" {
                    let mut collected = Vec::new();
                    i += 1;
                    while i < lines.len() {
                        let bl = lines[i];
                        if bl.is_empty() {
                            collected.push(String::new());
                            i += 1;
                            continue;
                        }
                        if bl.starts_with(' ') || bl.starts_with('\t') {
                            collected.push(bl.trim_start().to_string());
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    collected.join("\n")
                } else {
                    i += 1;
                    unquote(value).into_owned()
                };
                if key == "template" {
                    spell.template = block;
                } else {
                    spell.undo_template = Some(block);
                }
                continue;
            }
            _ => {
                // Unknown key — ignored for forward compat, like the
                // 6 sample files would expect (no extra metadata).
            }
        }
        i += 1;
    }
    spell.validate().map_err(|e| e.to_string())?;
    Ok(spell)
}

fn parse_param_item(item: &str) -> Result<Param, String> {
    let mut p = Param::default();
    // The buffer may contain multiple `key: value` pairs joined by
    // spaces (one per line of the YAML param record). Walk it and
    // extract a value for each known key.
    const KEYS: &[&str] = &["name", "prompt", "default", "required"];
    for key in KEYS {
        let needle = format!("{key}:");
        if let Some(idx) = item.find(&needle) {
            let after = &item[idx + needle.len()..];
            // Value runs until the next " <known-key>:" boundary or
            // end of input. Trim leading whitespace.
            let trimmed = after.trim_start();
            let mut end = trimmed.len();
            for next in KEYS {
                let marker = format!(" {next}:");
                if let Some(j) = trimmed.find(&marker) {
                    if j < end {
                        end = j;
                    }
                }
            }
            let value = trimmed[..end].trim_end();
            match *key {
                "name" => p.name = unquote(value).into_owned(),
                "prompt" => p.prompt = unquote(value).into_owned(),
                "default" => p.default = Some(unquote(value).into_owned()),
                "required" => p.required = matches!(value, "true" | "yes" | "1"),
                _ => {}
            }
        }
    }
    if p.name.is_empty() {
        return Err("param missing `name`".to_string());
    }
    Ok(p)
}

/// Strip a single pair of surrounding double or single quotes if present.
fn unquote(s: &str) -> std::borrow::Cow<'_, str> {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return std::borrow::Cow::Owned(s[1..s.len() - 1].to_string());
        }
    }
    std::borrow::Cow::Borrowed(s)
}

/// Registry backed by a directory on disk. Loads on construction and
/// hot-reloads on mtime change via `refresh_if_stale`. Mirrors the
/// style of `m5tui_profile::FileRegistry`.
pub struct FileRegistry {
    dir: PathBuf,
    inner: InMemoryRegistry,
    mtimes: HashMap<PathBuf, SystemTime>,
}

impl FileRegistry {
    pub fn new(dir: &Path) -> Result<Self, BookError> {
        let inner = load_book_dir(dir)?;
        let mtimes = collect_mtimes(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            inner,
            mtimes,
        })
    }

    pub fn registry(&self) -> &InMemoryRegistry {
        &self.inner
    }

    pub fn spell_count(&self) -> usize {
        self.inner.list().len()
    }

    /// Re-read any YAML that is new or whose mtime changed; drop
    /// spells whose file vanished. Returns `true` if anything changed.
    pub fn refresh_if_stale(&mut self) -> Result<bool, BookError> {
        let current = collect_mtimes(&self.dir)?;
        let mut changed = false;

        // New or modified files: re-parse and insert/replace.
        for (path, mtime) in &current {
            let stale = match self.mtimes.get(path) {
                Some(prev) => *prev != *mtime,
                None => true,
            };
            if !stale {
                continue;
            }
            let body = std::fs::read_to_string(path)
                .map_err(|e| BookError::io("read_to_string", path, e))?;
            let spell = parse_spell_yaml(&body).map_err(|message| BookError::Parse {
                path: path.clone(),
                line: 0,
                message,
            })?;
            // Remove old version under the same name (if any) before insert.
            if self.inner.spells.contains_key(&spell.name) {
                self.inner.spells.remove(&spell.name);
            }
            self.inner.insert(spell)?;
            changed = true;
        }

        // Deleted files: drop spells whose source path is gone.
        let removed: Vec<PathBuf> = self
            .mtimes
            .keys()
            .filter(|p| !current.contains_key(*p))
            .cloned()
            .collect();
        for path in &removed {
            // Map by the stem: spells are keyed by `name`, not filename.
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if self.inner.spells.remove(stem).is_some() {
                    changed = true;
                }
            }
        }

        if changed {
            self.mtimes = current;
        }
        Ok(changed)
    }
}

fn collect_mtimes(dir: &Path) -> Result<HashMap<PathBuf, SystemTime>, BookError> {
    let mut out = HashMap::new();
    let entries = std::fs::read_dir(dir).map_err(|e| BookError::io("read_dir", dir, e))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !(path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml")) {
            continue;
        }
        let mtime = entry
            .metadata()
            .map_err(|e| BookError::io("metadata", &path, e))?
            .modified()
            .map_err(|e| BookError::io("modified", &path, e))?;
        out.insert(path, mtime);
    }
    Ok(out)
}

impl BookRegistry for FileRegistry {
    fn list(&self) -> Vec<&Spell> {
        self.inner.list()
    }
    fn get(&self, name: &str) -> Option<&Spell> {
        self.inner.get(name)
    }
    fn add(&mut self, spell: Spell) -> Result<(), BookError> {
        self.inner.add(spell)
    }
    fn remove(&mut self, name: &str) -> Result<(), BookError> {
        self.inner.remove(name)
    }
    fn export_yaml(&self, name: &str) -> Result<String, BookError> {
        self.inner.export_yaml(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask_spell() -> Spell {
        Spell {
            name: "ask".to_string(),
            key: "a".to_string(),
            prefix: ";".to_string(),
            help: "Ask OMP a question".to_string(),
            params: vec![Param {
                name: "question".to_string(),
                prompt: "Question".to_string(),
                default: None,
                required: true,
            }],
            template: r##"omp ask "{question}""##.to_string(),
            undo_template: None,
        }
    }

    #[test]
    fn spell_expand_replaces_param() {
        let s = ask_spell();
        let mut values = HashMap::new();
        values.insert("question".to_string(), "what is rust?".to_string());
        assert_eq!(s.expand(&values), r#"omp ask "what is rust?""#);
    }

    #[test]
    fn spell_expand_leaves_missing_placeholders() {
        let s = ask_spell();
        assert_eq!(s.expand(&HashMap::new()), r#"omp ask "{question}""#);
    }

    #[test]
    fn registry_add_and_get() {
        let mut r = InMemoryRegistry::new();
        r.add(ask_spell()).unwrap_or_else(|e| panic!("{e}"));
        assert!(r.get("ask").is_some());
    }

    #[test]
    fn registry_rejects_duplicate() {
        let mut r = InMemoryRegistry::new();
        let s = ask_spell();
        r.add(s.clone()).unwrap_or_else(|e| panic!("{e}"));
        assert!(r.add(s).is_err());
    }

    #[test]
    fn undo_stack_lifo() {
        let mut u = UndoStack::new();
        u.push("undo a".to_string());
        u.push("undo b".to_string());
        assert_eq!(u.pop(), Some("undo b".to_string()));
        assert_eq!(u.pop(), Some("undo a".to_string()));
        assert!(u.is_empty());
    }

    #[test]
    fn export_yaml_contains_name() {
        let mut r = InMemoryRegistry::new();
        r.add(ask_spell()).unwrap_or_else(|e| panic!("{e}"));
        let yaml = r.export_yaml("ask").unwrap_or_else(|e| panic!("{e}"));
        assert!(yaml.contains("name: ask"));
        assert!(yaml.contains("template:"));
    }

    #[test]
    fn render_prompts_lists_required_marker() {
        let s = Spell {
            name: "ask".to_string(),
            key: "a".to_string(),
            prefix: ";".to_string(),
            help: "h".to_string(),
            params: vec![
                Param {
                    name: "q".into(),
                    prompt: "question".into(),
                    default: None,
                    required: true,
                },
                Param {
                    name: "lang".into(),
                    prompt: "language".into(),
                    default: Some("rust".into()),
                    required: false,
                },
            ],
            template: "{q} in {lang}".to_string(),
            undo_template: None,
        };
        let lines = s.render_prompts();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("*"));
        assert!(lines[1].contains("default: rust"));
        assert!(!lines[1].contains("*"));
    }

    #[test]
    fn validate_values_rejects_missing_required() {
        let s = ask_spell();
        let mut h = HashMap::new();
        h.insert("question".into(), "".into());
        assert!(s.validate_values(&h).is_err());
        h.insert("question".into(), "hi".into());
        assert!(s.validate_values(&h).is_ok());
    }

    #[test]
    fn values_with_defaults_fills_missing() {
        let s = Spell {
            name: "ask".to_string(),
            key: "a".to_string(),
            prefix: ";".to_string(),
            help: "h".to_string(),
            params: vec![
                Param {
                    name: "q".into(),
                    prompt: "?".into(),
                    default: None,
                    required: true,
                },
                Param {
                    name: "lang".into(),
                    prompt: "?".into(),
                    default: Some("rust".into()),
                    required: false,
                },
            ],
            template: "x".to_string(),
            undo_template: None,
        };
        let h = HashMap::new();
        let v = s.values_with_defaults(&h);
        assert_eq!(v.get("lang").map(String::as_str), Some("rust"));
        assert_eq!(v.get("q").map(String::as_str), Some(""));
    }

    #[test]
    fn author_editor_builds_spell() {
        let mut ed = AuthorEditor::new("ask", "a")
            .with_help("Ask OMP")
            .with_prefix(";")
            .with_template(r#"omp ask "{q}""#)
            .add_param("q", "question", true);
        ed.set_param("q", "what is rust?");
        let s = ed.commit().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.name, "ask");
        assert_eq!(s.params.len(), 1);
        assert_eq!(s.template, r#"omp ask "{q}""#);
    }

    #[test]
    fn author_editor_rejects_empty_template() {
        let ed = AuthorEditor::new("x", "x");
        assert!(ed.commit().is_err());
    }

    #[test]
    fn author_editor_prompt_for_and_value_of() {
        let mut ed =
            AuthorEditor::new("s", "k")
                .with_template("t")
                .add_param("color", "what color?", true);
        assert_eq!(ed.prompt_for("color").as_deref(), Some("what color?"));
        assert!(ed.value_of("color").is_none());
        ed.set_param("color", "blue");
        assert_eq!(ed.value_of("color"), Some("blue"));
        assert!(ed.prompt_for("missing").is_none());
    }
}
