//! Invariant checks for a parsed `Theme`. The parser enforces shape
//! (sections, types, hex syntax); `validate` enforces semantics
//! (lengths, ranges, palette completeness). Both run before a theme
//! is accepted into the live system, so a failure here is recoverable
//! and the editor can show the exact reason.

use crate::schema::Theme;
use crate::ThemeError;

/// Check that a parsed theme is safe to load. Returns `Ok(())` on a
/// valid theme, or `Err(ThemeError::Validation)` with a short reason.
///
/// Invariants:
/// - `name` is 1..32 chars (not empty, not absurdly long).
/// - `brightness` is 0..=100.
/// - `animation.level` is 0..=2.
/// - `palette` has all 10 swatches (the struct fields are required; the
///   parser populates them all if the section is present at all).
pub(crate) fn validate(theme: &Theme) -> Result<(), ThemeError> {
    if theme.name.is_empty() {
        return Err(ThemeError::Validation("name must not be empty".to_string()));
    }
    if theme.name.len() >= 32 {
        return Err(ThemeError::Validation(format!(
            "name length {} must be < 32",
            theme.name.len()
        )));
    }
    if theme.brightness > 100 {
        return Err(ThemeError::Validation(format!(
            "brightness {} must be <= 100",
            theme.brightness
        )));
    }
    if theme.animation.level > 2 {
        return Err(ThemeError::Validation(format!(
            "animation.level {} must be <= 2",
            theme.animation.level
        )));
    }
    // Palette completeness: every slot is a `Rgb565`, so there is no
    // "missing" state at the type level. We only need to guard against
    // accidental "transparent" zeros that would make text invisible.
    // We treat `bg == 0` AND `fg == 0` as a likely authoring error.
    if theme.palette.bg == theme.palette.fg && theme.palette.bg == crate::schema::Rgb565(0) {
        return Err(ThemeError::Validation(
            "palette bg and fg are both pure black; text will be invisible".to_string(),
        ));
    }
    Ok(())
}
