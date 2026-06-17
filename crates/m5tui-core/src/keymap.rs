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

use crate::app::Mode;
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
            't' => Some(KeyAction::OpenThemeEditor),
            'p' => Some(KeyAction::OpenProfilePicker),
            'b' => Some(KeyAction::OpenBook),
            'v' => Some(KeyAction::OpenVoice),
            'n' => Some(KeyAction::OpenFirstBoot),
            's' => Some(KeyAction::OpenSettings),
            'D' => Some(KeyAction::RunDoctor),
            'h' => Some(KeyAction::OpenHandoff),
            'A' => Some(KeyAction::OpenAbout),
            'm' => Some(KeyAction::OpenMemory),
            'w' => Some(KeyAction::SaveMemo),
            'L' => Some(KeyAction::OpenLogViewer),
            ';' => Some(KeyAction::Char(';')),
            c => Some(KeyAction::Char(c)),
        },
        None if new == ';' => Some(KeyAction::Palette),
        _ => Some(KeyAction::Char(new)),
    }
}
/// Resolve a printable char to a mode-specific key action. Returns
/// `Some(KeyAction)` only when the active mode has its own in-mode
/// binding for `c`; returns `None` when the caller should fall back
/// to `from_char` / `parse_chord`.
///
/// Bindings by mode:
/// - `Mode::LogViewer`:
///   - `j` -> `LogScrollUp` (one line older)
///   - `k` -> `LogScrollDown` (one line newer)
///   - `g` -> `LogScrollTop` (jump to oldest)
///   - `G` -> `LogScrollBottom` (jump to newest + re-enable follow)
///   - `f` -> `LogToggleFollow` (toggle follow-tail)
/// - `Mode::ProfilePicker`: `j`/`k` move the cursor down/up.
/// - `Mode::Book`: `j`/`k` move the cursor down/up.
/// - `Mode::Settings`: `j`/`k` move the cursor down/up; `-`/`+` adjust
///   the value under the cursor (brightness step / imu-wake cycle);
///   ` ` (space) toggles sound / cycles imu-wake.
pub fn keymap_for_mode(mode: Mode, c: char) -> Option<KeyAction> {
    match mode {
        Mode::LogViewer => match c {
            'j' => Some(KeyAction::LogScrollUp),
            'k' => Some(KeyAction::LogScrollDown),
            'g' => Some(KeyAction::LogScrollTop),
            'G' => Some(KeyAction::LogScrollBottom),
            'f' => Some(KeyAction::LogToggleFollow),
            _ => None,
        },
        Mode::ProfilePicker => match c {
            'j' => Some(KeyAction::ProfileDown),
            'k' => Some(KeyAction::ProfileUp),
            _ => None,
        },
        Mode::Book => match c {
            'j' => Some(KeyAction::BookDown),
            'k' => Some(KeyAction::BookUp),
            _ => None,
        },
        Mode::Settings => match c {
            'j' => Some(KeyAction::SettingsDown),
            'k' => Some(KeyAction::SettingsUp),
            '-' => Some(KeyAction::SettingsLeft),
            '+' => Some(KeyAction::SettingsRight),
            ' ' => Some(KeyAction::SettingsToggle),
            _ => None,
        },
        _ => None,
    }
}
/// State-aware chord parser. Falls back to `parse_chord` for the
/// state-less path, then layers on mode-specific overrides:
///
/// - `;?` while `mode == Mode::Help` -> `OpenAbout` (the user pressed
///   the help key from inside the help overlay, which the spec maps
///   to opening the about sheet).
pub fn parse_chord_with_mode(
    prev: Option<char>,
    new: char,
    mode: crate::app::Mode,
) -> Option<KeyAction> {
    if prev == Some(';') && new == '?' && mode == crate::app::Mode::Help {
        return Some(KeyAction::OpenAbout);
    }
    parse_chord(prev, new)
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
    fn chord_t_is_theme_editor() {
        assert_eq!(
            parse_chord(Some(';'), 't'),
            Some(KeyAction::OpenThemeEditor)
        );
    }

    #[test]
    fn chord_unknown_char_passes_through() {
        assert_eq!(parse_chord(Some(';'), 'x'), Some(KeyAction::Char('x')));
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

    #[test]
    fn chord_p_is_profile_picker() {
        assert_eq!(
            parse_chord(Some(';'), 'p'),
            Some(KeyAction::OpenProfilePicker)
        );
    }

    #[test]
    fn chord_b_is_book() {
        assert_eq!(parse_chord(Some(';'), 'b'), Some(KeyAction::OpenBook));
    }

    #[test]
    fn chord_v_is_voice() {
        assert_eq!(parse_chord(Some(';'), 'v'), Some(KeyAction::OpenVoice));
    }

    #[test]
    fn chord_n_is_first_boot() {
        assert_eq!(parse_chord(Some(';'), 'n'), Some(KeyAction::OpenFirstBoot));
    }

    #[test]
    fn chord_s_is_settings() {
        assert_eq!(parse_chord(Some(';'), 's'), Some(KeyAction::OpenSettings));
    }

    #[test]
    fn chord_capital_d_is_doctor() {
        assert_eq!(parse_chord(Some(';'), 'D'), Some(KeyAction::RunDoctor));
    }

    #[test]
    fn chord_h_is_handoff() {
        assert_eq!(parse_chord(Some(';'), 'h'), Some(KeyAction::OpenHandoff));
    }

    #[test]
    fn chord_m_is_memory() {
        assert_eq!(parse_chord(Some(';'), 'm'), Some(KeyAction::OpenMemory));
    }

    #[test]
    fn chord_w_is_save_memo() {
        assert_eq!(parse_chord(Some(';'), 'w'), Some(KeyAction::SaveMemo));
    }
    #[test]
    fn chord_capital_l_is_log_viewer() {
        assert_eq!(parse_chord(Some(';'), 'L'), Some(KeyAction::OpenLogViewer));
    }

    #[test]
    fn in_mode_log_j_is_scroll_up() {
        assert_eq!(
            keymap_for_mode(Mode::LogViewer, 'j'),
            Some(KeyAction::LogScrollUp),
        );
    }

    #[test]
    fn in_mode_log_k_is_scroll_down() {
        assert_eq!(
            keymap_for_mode(Mode::LogViewer, 'k'),
            Some(KeyAction::LogScrollDown),
        );
    }

    #[test]
    fn in_mode_log_lowercase_g_is_top() {
        assert_eq!(
            keymap_for_mode(Mode::LogViewer, 'g'),
            Some(KeyAction::LogScrollTop),
        );
    }

    #[test]
    fn in_mode_log_capital_g_is_bottom() {
        assert_eq!(
            keymap_for_mode(Mode::LogViewer, 'G'),
            Some(KeyAction::LogScrollBottom),
        );
    }

    #[test]
    fn in_mode_log_f_is_toggle_follow() {
        assert_eq!(
            keymap_for_mode(Mode::LogViewer, 'f'),
            Some(KeyAction::LogToggleFollow),
        );
    }

    #[test]
    fn in_mode_log_other_keys_pass_through() {
        assert_eq!(keymap_for_mode(Mode::LogViewer, 'x'), None);
        assert_eq!(keymap_for_mode(Mode::LogViewer, '?'), None);
    }

    #[test]
    fn in_mode_other_modes_have_no_overrides() {
        assert_eq!(keymap_for_mode(Mode::Cockpit, 'j'), None);
        assert_eq!(keymap_for_mode(Mode::Help, 'f'), None);
        assert_eq!(keymap_for_mode(Mode::Palette, 'g'), None);
    }

    #[test]
    fn chord_capital_a_is_open_about() {
        assert_eq!(parse_chord(Some(';'), 'A'), Some(KeyAction::OpenAbout),);
    }

    #[test]
    fn chord_question_mark_from_help_is_open_about() {
        // `;?` typed while the help overlay is open re-routes to the
        // about sheet instead of opening help again.
        assert_eq!(
            parse_chord_with_mode(Some(';'), '?', Mode::Help),
            Some(KeyAction::OpenAbout),
        );
    }

    #[test]
    fn chord_question_mark_from_cockpit_is_help() {
        // Outside the help overlay, `;?` still opens help.
        assert_eq!(
            parse_chord_with_mode(Some(';'), '?', Mode::Cockpit),
            Some(KeyAction::Help),
        );
    }
    #[test]
    fn settings_mode_j_is_down() {
        assert_eq!(
            keymap_for_mode(Mode::Settings, 'j'),
            Some(KeyAction::SettingsDown)
        );
    }

    #[test]
    fn settings_mode_k_is_up() {
        assert_eq!(
            keymap_for_mode(Mode::Settings, 'k'),
            Some(KeyAction::SettingsUp)
        );
    }

    #[test]
    fn settings_mode_plus_is_right() {
        assert_eq!(
            keymap_for_mode(Mode::Settings, '+'),
            Some(KeyAction::SettingsRight)
        );
    }

    #[test]
    fn settings_mode_minus_is_left() {
        assert_eq!(
            keymap_for_mode(Mode::Settings, '-'),
            Some(KeyAction::SettingsLeft)
        );
    }

    #[test]
    fn settings_mode_space_is_toggle() {
        assert_eq!(
            keymap_for_mode(Mode::Settings, ' '),
            Some(KeyAction::SettingsToggle)
        );
    }

    #[test]
    fn settings_mode_unrelated_chars_pass_through() {
        assert_eq!(keymap_for_mode(Mode::Settings, 'a'), None);
        assert_eq!(keymap_for_mode(Mode::Settings, '?'), None);
    }
}
