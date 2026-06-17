//! Book picker overlay — list `state.book_spells`, highlight the
//! cursor row, render a hint footer.
//!
//! Same shape as `super::profile_picker`, but the title is `Book`,
//! each row is `<key>  <help>`, and the hint ends in `cast` instead
//! of `select`.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// Row holding the title.
const TITLE_ROW: usize = 0;
/// First row of the body list.
const BODY_START: usize = 1;
/// Number of body rows (rows 1..=14 inclusive).
const BODY_ROWS: usize = 14;
/// Row holding the hint footer.
const HINT_ROW: usize = ROWS - 1;
/// Title text.
const TITLE: &str = "Book";
/// Hint footer text.
const HINT: &str = "j/k move  \u{23CE} cast  esc back";
/// Empty-state message when there are no spells.
const EMPTY: &str = "No spells \u{2014} run ;b with book/";

/// Render the book picker into the given frame. The widget reads
/// `state.book_spells` and `state.picker_index` but does not modify
/// `state`.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    let dim = theme.palette.dim.0;
    let ok = theme.palette.ok.0;

    // Clear the pane with a solid background.
    for row in 0..ROWS {
        for col in 0..COLS {
            frame.cells[row][col] = Cell {
                glyph: b' ',
                fg,
                bg,
                attrs: 0,
            };
        }
    }

    // Title row.
    write_str_colored(frame, TITLE_ROW, 0, TITLE, accent, bg);

    // Body rows. Scrolling so the highlighted row stays visible.
    if state.book_spells.is_empty() {
        write_str_colored(frame, BODY_START, 0, EMPTY, dim, bg);
    } else {
        let start = state.picker_index.saturating_sub(BODY_ROWS - 1);
        for (i, idx) in (start..state.book_spells.len()).take(BODY_ROWS).enumerate() {
            let row = BODY_START + i;
            if row >= HINT_ROW {
                break;
            }
            let spell = &state.book_spells[idx];
            let color = if idx == state.picker_index { ok } else { fg };
            let line = format!("{}  {}", spell.key, spell.help);
            write_line_colored(frame, row, 0, &line, color, bg);
        }
    }

    // Hint footer.
    write_str_colored(frame, HINT_ROW, 0, HINT, dim, bg);
}

/// Write `s` into the frame at `(row, col)`, truncated to `COLS`.
/// Non-printable bytes are rendered as `?`.
fn write_str_colored(frame: &mut Frame, row: usize, col: usize, s: &str, fg: u16, bg: u16) {
    if row >= ROWS {
        return;
    }
    for (i, b) in s.bytes().enumerate() {
        let c = col + i;
        if c >= COLS {
            break;
        }
        let glyph = if b.is_ascii_graphic() || b == b' ' {
            b
        } else {
            b'?'
        };
        frame.cells[row][c] = Cell {
            glyph,
            fg,
            bg,
            attrs: 0,
        };
    }
}

/// Write `s` into the frame at `(row, 0)`, truncated to `COLS`.
fn write_line_colored(frame: &mut Frame, row: usize, col: usize, s: &str, fg: u16, bg: u16) {
    write_str_colored(frame, row, col, s, fg, bg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::SpellSummary;

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn two_spells() -> Vec<SpellSummary> {
        vec![
            SpellSummary {
                id: "s1".into(),
                key: "ask".into(),
                help: "Ask the agent a question".into(),
            },
            SpellSummary {
                id: "s2".into(),
                key: "plan".into(),
                help: "Draft a plan for the task".into(),
            },
        ]
    }

    #[test]
    fn title_renders_in_accent_color() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState {
            book_spells: two_spells(),
            ..Default::default()
        };
        render(&mut f, &state, &theme);
        // Row 0: "Book" in accent.
        assert_eq!(f.cells[0][0].glyph, b'B');
        assert_eq!(f.cells[0][0].fg, theme.palette.accent.0);
    }

    #[test]
    fn both_spell_keys_visible() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState {
            book_spells: two_spells(),
            ..Default::default()
        };
        render(&mut f, &state, &theme);
        // Row 1: "ask ..." -> 'a' at col 0.
        assert_eq!(f.cells[1][0].glyph, b'a');
        // Row 2: "plan ..." -> 'p' at col 0.
        assert_eq!(f.cells[2][0].glyph, b'p');
    }

    #[test]
    fn empty_spells_shows_empty_message() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState::default();
        render(&mut f, &state, &theme);
        // Row 1 starts with 'N' from "No spells".
        assert_eq!(f.cells[1][0].glyph, b'N');
        assert_eq!(f.cells[1][0].fg, theme.palette.dim.0);
    }

    #[test]
    fn hint_footer_in_dim() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState::default();
        render(&mut f, &state, &theme);
        // Row 15 (last row) starts with 'j'.
        assert_eq!(f.cells[ROWS - 1][0].glyph, b'j');
        assert_eq!(f.cells[ROWS - 1][0].fg, theme.palette.dim.0);
    }
}
