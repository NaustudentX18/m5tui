//! `m5tui-book` — Book of Commands.
//!
//! Spells are data, not code. Each spell has a key, prefix, parameter
//! prompts, a template, and an optional undo template. Templates expand
//! `{param}` placeholders. M5a scope is the registry + expander + undo
//! stack; real on-device author mode is deferred.

use std::collections::HashMap;

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
    MissingParam(String),
}

impl std::fmt::Display for BookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "invalid: {s}"),
            Self::NotFound(s) => write!(f, "not found: {s}"),
            Self::Duplicate(s) => write!(f, "duplicate: {s}"),
            Self::MissingParam(s) => write!(f, "missing param: {s}"),
        }
    }
}

impl std::error::Error for BookError {}

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

/// Load all `.yaml` files under a directory into a registry.
pub fn load_book_dir(_dir: &std::path::Path) -> Result<InMemoryRegistry, BookError> {
    // M5a stub: real YAML loader deferred. Return empty registry.
    Ok(InMemoryRegistry::new())
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
