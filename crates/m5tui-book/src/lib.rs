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
}
