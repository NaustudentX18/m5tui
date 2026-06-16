//! Keymap for the M5Stack Cardputer-Adv keyboard.
//!
//! The Cardputer-Adv has a 56-key matrix laid out in 4 rows of 14 keys. The
//! `LAYOUT` constant is the static keymap the framework consumes to turn a
//! scan code into a printable character. The semicolon key (`;`) at row 2,
//! column 10 is the "command prefix" key — the user types `;` followed by
//! another key to invoke a verb (`;?` for help, `;/` for palette).
//!
//! `parse_chord` is the second half of the input pipeline: the caller feeds
//! it the previous unmatched `;` and the next printable char, and it
//! returns a semantic `KeyAction`. When no `;` is pending, the lone `;` is
//! reported as `KeyAction::Palette` so the input layer above can decide
//! whether to start a chord.

use crate::event::KeyAction;

/// Static 4x14 keyboard layout for the Cardputer-Adv. The fifth column
/// (index 13) of every row is a modifier slot; in M1 the modifier is
/// unused so the entries there are space, but the matrix is still 14
/// columns wide to match the hardware.
pub const LAYOUT: [[char; 14]; 4] = [
    // Row 0: top number row, then the backtick/minus/equals/backspace slot.
    [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '\'', '-', '=', '\\',
    ],
    // Row 1: top alphas + bracket slot.
    [
        'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', ' ', ' ',
    ],
    // Row 2: home alphas + the all-important `;` chord key.
    [
        'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', ' ', ' ', ' ',
    ],
    // Row 3: bottom alphas + punctuation.
    [
        'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', ' ', ' ', ' ', ' ',
    ],
];

/// Resolve a printable char to a semantic key action. This is the
/// "no-chord-pending" path: `;` alone is reported as `KeyAction::Palette`
/// so the framework can stash it and call `parse_chord` on the next key.
pub fn from_char(c: char) -> Option<KeyAction> {
    match c {
        ';' => Some(KeyAction::Palette),
        c if c.is_ascii_graphic() || c == ' ' => Some(KeyAction::Char(c)),
        _ => None,
    }
}

/// Resolve a printable char to a semantic key action, optionally with a
/// pending chord prefix. This is the single entry point the input layer
/// calls; `from_char` is a thin wrapper for the no-prefix case.
///
/// - Bare `;` with no pending prefix -> `KeyAction::Palette` (open the
///   command palette). The caller is expected to stash the `;` so the
///   next key is dispatched as a chord.
/// - With a pending `;` prefix:
///   - `;?` -> `KeyAction::Help`
///   - `;/` -> `KeyAction::Palette` (re-enter the palette with the
///     prefix consumed)
///   - `;;` -> `KeyAction::Char(';')` (escape: the user wanted a
///     literal semicolon)
///   - any other `new` -> `KeyAction::Char(new)` (the `;` is consumed
///     as the prefix and the new char passes through as a normal char)
pub fn parse_chord(prev: Option<char>, new: char) -> Option<KeyAction> {
    match prev {
        Some(';') => match new {
            '?' => Some(KeyAction::Help),
            '/' => Some(KeyAction::Palette),
            ';' => Some(KeyAction::Char(';')),
            c => Some(KeyAction::Char(c)),
        },
        None if new == ';' => Some(KeyAction::Palette),
        _ => Some(KeyAction::Char(new)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_4_rows_of_14() {
        assert_eq!(LAYOUT.len(), 4);
        for row in &LAYOUT {
            assert_eq!(row.len(), 14);
        }
    }

    #[test]
    fn layout_semicolon_position() {
        // The chord key sits at row 2, column 9.
        assert_eq!(LAYOUT[2][9], ';');
    }

    #[test]
    fn from_char_letter_is_char_action() {
        assert_eq!(from_char('h'), Some(KeyAction::Char('h')));
    }

    #[test]
    fn from_char_semicolon_is_palette() {
        assert_eq!(from_char(';'), Some(KeyAction::Palette));
    }

    #[test]
    fn from_char_rejects_non_graphic() {
        assert_eq!(from_char('\n'), None);
        assert_eq!(from_char('\t'), None);
    }

    #[test]
    fn chord_question_mark_is_help() {
        assert_eq!(parse_chord(Some(';'), '?'), Some(KeyAction::Help));
    }

    #[test]
    fn chord_slash_is_palette() {
        assert_eq!(parse_chord(Some(';'), '/'), Some(KeyAction::Palette));
    }

    #[test]
    fn chord_double_semicolon_is_literal() {
        assert_eq!(parse_chord(Some(';'), ';'), Some(KeyAction::Char(';')));
    }

    #[test]
    fn chord_unknown_char_passes_through() {
        assert_eq!(parse_chord(Some(';'), 'h'), Some(KeyAction::Char('h')));
    }

    #[test]
    fn no_prefix_passes_char_through() {
        assert_eq!(parse_chord(None, 'h'), Some(KeyAction::Char('h')));
    }

    #[test]
    fn no_prefix_semicolon_is_palette() {
        // A bare `;` with no chord prefix opens the palette; the caller
        // is then expected to stash the `;` and dispatch the next key
        // through `parse_chord(Some(';'), next)`.
        assert_eq!(parse_chord(None, ';'), Some(KeyAction::Palette));
    }
}
