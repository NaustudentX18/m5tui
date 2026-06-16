//! `m5tui-themes` — theme schema, hand-rolled YAML parser, validator,
//! and the six built-in themes that ship with the binary.
//!
//! ## Public surface
//!
//! - [`Theme`] and its sub-types ([`ThemePalette`], [`ThemeGlyphs`],
//!   [`ThemeLayout`], [`ThemeAnimation`], [`ThemeSound`], [`Rgb565`]).
//! - The enum choices ([`GlyphSet`], [`ScrollbarStyle`], [`CursorStyle`],
//!   [`Density`]).
//! - [`BUILTINS`] — the list of `(name, yaml_source)` pairs.
//! - [`BUILTIN_NAMES`] — just the names, in picker order.
//! - [`parse`] — parse a YAML string into a `Theme`.
//! - [`validate`] — semantic checks (name length, ranges, etc.).
//! - [`parse_validated`] — parse + validate in one call.
//! - [`builtin`] — fetch one built-in by name.
//!
//! All errors are surfaced as `ThemeError`; the parser uses the
//! `Yaml` variant (with a 1-based line number) and `validate` uses the
//! `Validation` variant. The editor can map both back to a user-
//! friendly overlay.

pub mod builtins;
pub mod parser;
pub mod schema;
pub mod validate;

pub use builtins::{BUILTINS, BUILTIN_NAMES};
pub use schema::{
    CursorStyle, Density, GlyphSet, Rgb565, ScrollbarStyle, Theme, ThemeAnimation, ThemeGlyphs,
    ThemeLayout, ThemePalette, ThemeSound,
};

use std::fmt;

/// The single error type for the crate. `Yaml` carries a 1-based line
/// number; `Validation` carries a short human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    /// Structural parse error. The line number points at the first
    /// offending character; `reason` is a human-readable hint.
    Yaml { line: usize, reason: String },
    /// Semantic error detected by `validate`. A `Theme` produced by a
    /// successful `parse` is then checked here before use.
    Validation(String),
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml { line, reason } => write!(f, "yaml error at line {line}: {reason}"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
        }
    }
}

impl std::error::Error for ThemeError {}

/// Parse a theme YAML string into a [`Theme`]. Returns
/// [`ThemeError::Yaml`] for any structural problem and
/// [`ThemeError::Validation`] for any semantic check (name length,
/// brightness range, animation level). The two are combined so callers
/// can simply `let t = parse(src)?;` and trust the result.
pub fn parse(yaml: &str) -> Result<Theme, ThemeError> {
    let t = parser::parse(yaml)?;
    validate::validate(&t)?;
    Ok(t)
}

/// Semantic check on an already-parsed theme. Returns `Ok(())` or
/// [`ThemeError::Validation`] with a short reason. Exposed so callers
/// that already have a `Theme` in hand (e.g. from a live edit) can
/// re-check it without re-parsing the YAML.
pub fn validate(theme: &Theme) -> Result<(), ThemeError> {
    validate::validate(theme)
}

/// Convenience: parse + validate in one call. Equivalent to
/// `parse(yaml)` but more self-documenting at the call site.
pub fn parse_validated(yaml: &str) -> Result<Theme, ThemeError> {
    parse(yaml)
}

/// Look up a built-in theme by name (e.g. `"coldwire"`). Re-parses on
/// every call; the parser is allocation-light and the YAML is short
/// (~700 bytes), so caching is not worth the complexity. Returns
/// `None` when the name does not match any of the six built-ins.
pub fn builtin(name: &str) -> Option<Theme> {
    for (n, src) in BUILTINS {
        if *n == name {
            return parse(src).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_lookup_returns_some_for_coldwire() {
        let t = builtin("coldwire");
        assert!(t.is_some());
        let Some(t) = t else {
            panic!("builtin(\"coldwire\") returned None");
        };
        assert_eq!(t.name, "coldwire");
    }

    #[test]
    fn builtin_lookup_returns_none_for_unknown() {
        assert!(builtin("nonexistent").is_none());
        assert!(builtin("").is_none());
    }

    #[test]
    fn rgb565_round_trip_for_coldwire_accent() {
        // #00d8ff -> R=0, G=0xd8, B=0xff
        let c = Rgb565::from_rgb888(0x00, 0xd8, 0xff);
        // (0 >> 3) << 11 = 0
        // (0xd8 >> 2) << 5 = 0x36 << 5 = 0x06C0
        // (0xff >> 3) = 0x1F
        // 0x06C0 | 0x1F = 0x06DF
        assert_eq!(c.raw(), 0x06DF);
    }

    #[test]
    fn rgb565_to_rgb888_round_trip_is_approximate() {
        // 565 only has 5/6/5 bits, so we expect truncation.
        let c = Rgb565::from_rgb888(0xAB, 0xCD, 0xEF);
        let (r, g, b) = c.to_rgb888();
        // 0xAB = 1010_1011 -> top 5 = 0x15 = 21
        // Reconstructed: (21 << 3) | (21 >> 2) = 168 | 5 = 173 = 0xAD
        assert_eq!(r, 0xAD);
        // 0xCD = 1100_1101 -> top 6 = 0x33 = 51
        // Reconstructed: (51 << 2) | (51 >> 4) = 204 | 3 = 207 = 0xCF
        assert_eq!(g, 0xCF);
        // 0xEF = 1110_1111 -> top 5 = 0x1D = 29
        // Reconstructed: (29 << 3) | (29 >> 2) = 232 | 7 = 239 = 0xEF
        assert_eq!(b, 0xEF);
    }

    #[test]
    fn theme_error_display_yaml() {
        let e = ThemeError::Yaml {
            line: 7,
            reason: "bogus".to_string(),
        };
        assert_eq!(e.to_string(), "yaml error at line 7: bogus");
    }

    #[test]
    fn theme_error_display_validation() {
        let e = ThemeError::Validation("brightness out of range".to_string());
        assert_eq!(e.to_string(), "validation error: brightness out of range");
    }
}
