//! Hand-rolled YAML parser. The format is intentionally narrow:
//!
//! ```yaml
//! name: coldwire              # top-level scalar
//! brightness: 60              # u8
//! palette:                    # section header
//!   bg: "#0a1428"             # nested scalar
//!   accent: "#00d8ff"         # nested scalar
//! glyphs:
//!   box: heavy
//!   scrollbar: block
//! ```
//!
//! Indentation is exactly 2 spaces. Comments start with `#` and are
//! ignored. Strings may be bare or quoted with `"`. Hex colors are
//! 7-char `"#rrggbb"` literals. The parser is line-oriented; no
//! multi-line strings, no flow style, no anchors.
//!
//! Errors carry the 1-based line number and a human-readable reason so
//! the editor can highlight the offending line.

use crate::schema::*;
use crate::ThemeError;

/// Parse a complete theme YAML document and return a fully populated
/// `Theme`. Any structural error (unknown section, missing required
/// field, bad hex) becomes a `ThemeError::Yaml` with a line number.
pub(crate) fn parse(yaml: &str) -> Result<Theme, ThemeError> {
    // Top-level scalars and section pointers.
    let mut name: Option<String> = None;
    let mut author = String::new();
    let mut brightness: Option<u8> = None;

    // `None` between top-level keys, `Some(s)` inside a section. Reset
    // to `None` whenever we see a top-level key (so e.g. a stray
    // indented line after `brightness:` is rejected).
    let mut section: Option<Section> = None;

    // In-progress section payloads.
    let mut b = Builder::default();

    for (idx, raw_line) in yaml.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line);
        if line.trim().is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("  ") {
            // Indented: must be inside a section.
            let sec = section.as_ref().ok_or_else(|| ThemeError::Yaml {
                line: line_no,
                reason: "indented key with no parent section".to_string(),
            })?;
            let body = rest.trim();
            let (k, v) = split_kv(body, line_no)?;
            apply_nested(sec, k, v, line_no, &mut b)?;
        } else {
            // Top-level.
            let trimmed = line.trim();
            if let Some(header) = trimmed.strip_suffix(':') {
                section = Some(map_section(header.trim(), line_no)?);
                continue;
            }
            section = None;
            let (k, v) = split_kv(trimmed, line_no)?;
            match k {
                "name" => name = Some(v.to_string()),
                "author" => author = v.to_string(),
                "brightness" => {
                    let n: i64 = v.parse().map_err(|_| ThemeError::Yaml {
                        line: line_no,
                        reason: format!("brightness must be an integer, got {v:?}"),
                    })?;
                    if !(0..=100).contains(&n) {
                        return Err(ThemeError::Yaml {
                            line: line_no,
                            reason: format!("brightness {n} out of range 0..=100"),
                        });
                    }
                    brightness = Some(n as u8);
                }
                _ => {
                    return Err(ThemeError::Yaml {
                        line: line_no,
                        reason: format!("unknown top-level key: {k:?}"),
                    });
                }
            }
        }
    }

    // Required-field checks. Only `name` and `palette` are mandatory;
    // every other section has a sensible default below.
    let name = name.ok_or_else(|| ThemeError::Yaml {
        line: 1,
        reason: "missing required field: name".to_string(),
    })?;
    let palette = b.palette.ok_or_else(|| ThemeError::Yaml {
        line: 1,
        reason: "missing required section: palette".to_string(),
    })?;

    Ok(Theme {
        name,
        author,
        palette,
        glyphs: b.glyphs.unwrap_or_else(default_glyphs),
        layout: b.layout.unwrap_or_else(default_layout),
        animation: b.animation.unwrap_or_else(default_animation),
        sound: b.sound.unwrap_or_else(ThemeSound::default_on),
        brightness: brightness.unwrap_or(60),
    })
}

/// Nested section we are currently inside. Only one level of nesting
/// exists in the schema, so a flat enum suffices.
#[derive(Debug, Clone, Copy)]
enum Section {
    Palette,
    Glyphs,
    Layout,
    Animation,
    Sound,
}

fn map_section(name: &str, line: usize) -> Result<Section, ThemeError> {
    match name {
        "palette" => Ok(Section::Palette),
        "glyphs" => Ok(Section::Glyphs),
        "layout" => Ok(Section::Layout),
        "animation" => Ok(Section::Animation),
        "sound" => Ok(Section::Sound),
        other => Err(ThemeError::Yaml {
            line,
            reason: format!("unknown section: {other:?}"),
        }),
    }
}

/// In-progress parse state for the five nested sections. Each slot
/// starts as `None` and becomes `Some` the first time a key inside it
/// is seen.
#[derive(Default)]
struct Builder {
    palette: Option<ThemePalette>,
    glyphs: Option<ThemeGlyphs>,
    layout: Option<ThemeLayout>,
    animation: Option<ThemeAnimation>,
    sound: Option<ThemeSound>,
}

/// Apply one nested key/value to the right slot in `b`. Reads the
/// `section` discriminator, dispatches to the right handler.
fn apply_nested(
    section: &Section,
    k: &str,
    v: &str,
    line: usize,
    b: &mut Builder,
) -> Result<(), ThemeError> {
    match section {
        Section::Palette => {
            let p = b.palette.get_or_insert_with(default_palette);
            let rgb = parse_hex(v, line, k)?;
            match k {
                "bg" => p.bg = rgb,
                "fg" => p.fg = rgb,
                "accent" => p.accent = rgb,
                "dim" => p.dim = rgb,
                "warn" => p.warn = rgb,
                "err" => p.err = rgb,
                "ok" => p.ok = rgb,
                "sel_bg" => p.sel_bg = rgb,
                "sel_fg" => p.sel_fg = rgb,
                "prompt" => p.prompt = rgb,
                other => {
                    return Err(ThemeError::Yaml {
                        line,
                        reason: format!("unknown palette key: {other:?}"),
                    });
                }
            }
        }
        Section::Glyphs => {
            let g = b.glyphs.get_or_insert_with(default_glyphs);
            match k {
                "box" => g.box_ = map_glyph_set(v, line)?,
                "scrollbar" => g.scrollbar = map_scrollbar(v, line)?,
                "cursor" => g.cursor = map_cursor(v, line)?,
                other => {
                    return Err(ThemeError::Yaml {
                        line,
                        reason: format!("unknown glyphs key: {other:?}"),
                    });
                }
            }
        }
        Section::Layout => {
            let l = b.layout.get_or_insert_with(default_layout);
            match k {
                "density" => l.density = map_density(v, line)?,
                "show_clock" => l.show_clock = parse_bool(v, line, k)?,
                "show_synthwave" => l.show_synthwave = parse_bool(v, line, k)?,
                other => {
                    return Err(ThemeError::Yaml {
                        line,
                        reason: format!("unknown layout key: {other:?}"),
                    });
                }
            }
        }
        Section::Animation => {
            let a = b.animation.get_or_insert_with(default_animation);
            match k {
                "level" => a.level = parse_u8_strict(v, line, k)?,
                "scanline" => a.scanline = parse_bool(v, line, k)?,
                "phosphor_decay" => a.phosphor_decay = parse_bool(v, line, k)?,
                "glitch_on_event" => a.glitch_on_event = parse_bool(v, line, k)?,
                other => {
                    return Err(ThemeError::Yaml {
                        line,
                        reason: format!("unknown animation key: {other:?}"),
                    });
                }
            }
        }
        Section::Sound => {
            let s = b.sound.get_or_insert_with(ThemeSound::default_on);
            match k {
                "enabled" => s.enabled = parse_bool(v, line, k)?,
                "boot" => s.boot = unquote(v).to_string(),
                "click" => s.click = unquote(v).to_string(),
                "arp" => s.arp = unquote(v).to_string(),
                other => {
                    return Err(ThemeError::Yaml {
                        line,
                        reason: format!("unknown sound key: {other:?}"),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Split `key: value` at the first `:` and return trimmed halves. Errors
/// if no `:` is present. The value may be empty (a bare `key:` would
/// have been caught as a section header above).
fn split_kv(s: &str, line: usize) -> Result<(&str, &str), ThemeError> {
    let (k, v) = s.split_once(':').ok_or_else(|| ThemeError::Yaml {
        line,
        reason: format!("expected `key: value`, got: {s:?}"),
    })?;
    Ok((k.trim(), v.trim()))
}

/// Strip a `# …` comment. The `#` must be preceded by whitespace or be
/// at the start of the line. Quoted strings are not handled because the
/// schema has no `#` characters inside string values.
fn strip_comment(line: &str) -> String {
    if let Some(hash_at) = find_comment_start(line) {
        line[..hash_at].to_string()
    } else {
        line.to_string()
    }
}

fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'#' {
            // Must be at start, or preceded by whitespace.
            if i == 0 || bytes[i - 1].is_ascii_whitespace() {
                return Some(i);
            }
        }
    }
    None
}

/// Parse a `"#rrggbb"` string. Returns the raw 6 hex digits as three
/// `u8`s, or a `ThemeError::Yaml` if the string is malformed.
fn parse_hex(s: &str, line: usize, key: &str) -> Result<Rgb565, ThemeError> {
    let s = unquote(s).trim();
    let body = s.strip_prefix('#').ok_or_else(|| ThemeError::Yaml {
        line,
        reason: format!("{key}: expected `#rrggbb`, got {s:?}"),
    })?;
    if body.len() != 6 {
        return Err(ThemeError::Yaml {
            line,
            reason: format!("{key}: expected 6 hex digits, got {} in {s:?}", body.len()),
        });
    }
    let r = u8::from_str_radix(&body[0..2], 16).map_err(|_| hex_err(line, key, s))?;
    let g = u8::from_str_radix(&body[2..4], 16).map_err(|_| hex_err(line, key, s))?;
    let b = u8::from_str_radix(&body[4..6], 16).map_err(|_| hex_err(line, key, s))?;
    Ok(Rgb565::from_rgb888(r, g, b))
}

fn hex_err(line: usize, key: &str, s: &str) -> ThemeError {
    ThemeError::Yaml {
        line,
        reason: format!("{key}: invalid hex color {s:?}"),
    }
}

/// Parse a boolean. Accepts `true`/`false` and `yes`/`no`. Anything
/// else is an error so typos don't silently default to `false`.
fn parse_bool(s: &str, line: usize, key: &str) -> Result<bool, ThemeError> {
    match s.trim() {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        other => Err(ThemeError::Yaml {
            line,
            reason: format!("{key}: expected true|false, got {other:?}"),
        }),
    }
}

/// Parse a `u8` (0..=255). Range checks for specific fields (level ≤ 2,
/// brightness ≤ 100) happen at the call site.
fn parse_u8_strict(s: &str, line: usize, key: &str) -> Result<u8, ThemeError> {
    let n: i64 = s.trim().parse().map_err(|_| ThemeError::Yaml {
        line,
        reason: format!("{key}: expected integer, got {s:?}"),
    })?;
    if !(0..=255).contains(&n) {
        return Err(ThemeError::Yaml {
            line,
            reason: format!("{key}: {n} out of range 0..=255"),
        });
    }
    Ok(n as u8)
}

fn map_glyph_set(v: &str, line: usize) -> Result<GlyphSet, ThemeError> {
    match v.trim() {
        "ascii" | "single" => Ok(GlyphSet::Ascii),
        "heavy" => Ok(GlyphSet::Heavy),
        "double" => Ok(GlyphSet::Double),
        "rounded" => Ok(GlyphSet::Rounded),
        other => Err(ThemeError::Yaml {
            line,
            reason: format!("unknown box glyph set: {other:?}"),
        }),
    }
}

fn map_scrollbar(v: &str, line: usize) -> Result<ScrollbarStyle, ThemeError> {
    match v.trim() {
        "block" => Ok(ScrollbarStyle::Block),
        "thin" => Ok(ScrollbarStyle::Thin),
        "arrow" | "line" => Ok(ScrollbarStyle::Arrow),
        other => Err(ThemeError::Yaml {
            line,
            reason: format!("unknown scrollbar style: {other:?}"),
        }),
    }
}

fn map_cursor(v: &str, line: usize) -> Result<CursorStyle, ThemeError> {
    match v.trim() {
        "block" => Ok(CursorStyle::Block),
        "underline" => Ok(CursorStyle::Underline),
        "bar" => Ok(CursorStyle::Bar),
        other => Err(ThemeError::Yaml {
            line,
            reason: format!("unknown cursor style: {other:?}"),
        }),
    }
}

fn map_density(v: &str, line: usize) -> Result<Density, ThemeError> {
    match v.trim() {
        "compact" => Ok(Density::Compact),
        "cozy" => Ok(Density::Cozy),
        "comfy" => Ok(Density::Comfy),
        other => Err(ThemeError::Yaml {
            line,
            reason: format!("unknown density: {other:?}"),
        }),
    }
}

/// Strip a single pair of double quotes if present. The schema's
/// filename / path values may be either bare or quoted.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn default_palette() -> ThemePalette {
    ThemePalette {
        bg: Rgb565(0),
        fg: Rgb565(0xFFFF),
        accent: Rgb565(0xFFFF),
        dim: Rgb565(0x7BEF),
        warn: Rgb565(0xFD20),
        err: Rgb565(0xF800),
        ok: Rgb565(0x07E0),
        sel_bg: Rgb565(0x0000),
        sel_fg: Rgb565(0xFFFF),
        prompt: Rgb565(0xFFFF),
    }
}

fn default_glyphs() -> ThemeGlyphs {
    ThemeGlyphs {
        box_: GlyphSet::Heavy,
        scrollbar: ScrollbarStyle::Block,
        cursor: CursorStyle::Block,
    }
}

fn default_layout() -> ThemeLayout {
    ThemeLayout {
        density: Density::Compact,
        show_clock: false,
        show_synthwave: false,
    }
}

fn default_animation() -> ThemeAnimation {
    ThemeAnimation {
        level: 0,
        scanline: false,
        phosphor_decay: false,
        glitch_on_event: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::BUILTINS;

    #[test]
    fn parse_empty_string_is_err() {
        let r = parse("");
        assert!(r.is_err(), "expected error for empty input");
    }

    #[test]
    fn parse_whitespace_only_is_err() {
        let r = parse("   \n\n   \n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_unknown_top_level_key_is_err() {
        let r = parse("name: x\nbogus_section: 1\n");
        assert!(r.is_err(), "expected error for unknown top-level key");
    }

    #[test]
    fn parse_indented_without_section_is_err() {
        let r = parse("name: x\n  indent_without_parent: 1\n");
        assert!(
            r.is_err(),
            "expected error for indented key with no section"
        );
    }

    #[test]
    fn parse_brightness_out_of_range_is_err() {
        let r = parse("name: x\nbrightness: 200\n");
        assert!(r.is_err());
        // Range check is at the parser level (string -> i64). The
        // error message should mention `brightness`.
        let msg = match r {
            Err(e) => format!("{e}"),
            Ok(_) => panic!("parse should have failed for brightness 200"),
        };
        assert!(
            msg.contains("brightness"),
            "expected error mentioning brightness, got: {msg}"
        );
    }

    #[test]
    fn parse_missing_name_is_err() {
        let r = parse("brightness: 60\npalette:\n  bg: \"#000000\"\n  fg: \"#ffffff\"\n  accent: \"#00d8ff\"\n  dim: \"#888888\"\n  warn: \"#ffaa00\"\n  err: \"#ff3050\"\n  ok: \"#44ee88\"\n  sel_bg: \"#1a3868\"\n  sel_fg: \"#ffffff\"\n  prompt: \"#00d8ff\"\n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_missing_palette_is_err() {
        let r = parse("name: x\n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_bad_hex_is_err() {
        let r = parse("name: x\npalette:\n  bg: \"#zzzzzz\"\n  fg: \"#ffffff\"\n  accent: \"#00d8ff\"\n  dim: \"#888888\"\n  warn: \"#ffaa00\"\n  err: \"#ff3050\"\n  ok: \"#44ee88\"\n  sel_bg: \"#1a3868\"\n  sel_fg: \"#ffffff\"\n  prompt: \"#00d8ff\"\n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_short_hex_is_err() {
        let r = parse("name: x\npalette:\n  bg: \"#fff\"\n  fg: \"#ffffff\"\n  accent: \"#00d8ff\"\n  dim: \"#888888\"\n  warn: \"#ffaa00\"\n  err: \"#ff3050\"\n  ok: \"#44ee88\"\n  sel_bg: \"#1a3868\"\n  sel_fg: \"#ffffff\"\n  prompt: \"#00d8ff\"\n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_unknown_palette_key_is_err() {
        let r = parse("name: x\npalette:\n  bg: \"#000000\"\n  evil: \"#ffffff\"\n  accent: \"#00d8ff\"\n  dim: \"#888888\"\n  warn: \"#ffaa00\"\n  err: \"#ff3050\"\n  ok: \"#44ee88\"\n  sel_bg: \"#1a3868\"\n  sel_fg: \"#ffffff\"\n  prompt: \"#00d8ff\"\n");
        assert!(r.is_err());
    }

    #[test]
    fn parse_unknown_section_is_err() {
        let r = parse("name: x\npalette:\n  bg: \"#000000\"\nfoo:\n  bar: 1\n");
        assert!(r.is_err());
    }

    #[test]
    fn all_builtins_parse_ok() {
        for (name, src) in BUILTINS {
            let t = parse(src).unwrap_or_else(|e| {
                panic!("builtin {name} failed to parse: {e}");
            });
            assert_eq!(t.name, *name, "name mismatch in {name}.yaml");
        }
    }

    #[test]
    fn comments_are_ignored() {
        let src = "# header comment\nname: x # trailing comment\n# palette:\npalette:\n  # bg slot\n  bg: \"#000000\"\n  fg: \"#ffffff\"\n  accent: \"#00d8ff\"\n  dim: \"#888888\"\n  warn: \"#ffaa00\"\n  err: \"#ff3050\"\n  ok: \"#44ee88\"\n  sel_bg: \"#1a3868\"\n  sel_fg: \"#ffffff\"\n  prompt: \"#00d8ff\"\n";
        let t = parse(src).unwrap_or_else(|e| panic!("parse should succeed despite comments: {e}"));
        assert_eq!(t.name, "x");
        assert_eq!(t.palette.bg, Rgb565(0));
    }

    #[test]
    fn hex_hash_in_middle_of_value_is_kept() {
        // `#` mid-value with non-whitespace prefix is content, not a comment.
        let src = "name: x\npalette:\n  bg: \"#000000\"\n  fg: \"#ffffff\"\n  accent: \"#00d8ff\"\n  dim: \"#888888\"\n  warn: \"#ffaa00\"\n  err: \"#ff3050\"\n  ok: \"#44ee88\"\n  sel_bg: \"#1a3868\"\n  sel_fg: \"#ffffff\"\n  prompt: \"#00d8ff\"\n";
        let t = parse(src).unwrap_or_else(|e| panic!("parse should succeed: {e}"));
        assert_eq!(
            t.palette.accent.raw(),
            Rgb565::from_rgb888(0, 0xd8, 0xff).raw()
        );
    }
}
