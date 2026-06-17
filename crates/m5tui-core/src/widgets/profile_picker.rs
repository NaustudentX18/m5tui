//! Profile picker overlay — list `state.profiles`, highlight the
//! cursor row, render a hint footer.
//!
//! Layout (40 cols × 16 rows):
//!
//! ```text
//!   row  0: title  "Profiles" in accent
//!   rows 1-14: body, scrolling so the highlighted row stays visible
//!              Each row: "<label>  <host>" (truncated to 40 cols).
//!              Highlighted row uses theme.palette.ok as fg.
//!              Empty state: "No profiles — run ;n" in theme.palette.dim.
//!   row 15: hint "j/k move  ⏎ select  esc back" in dim
//! ```

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
const TITLE: &str = "Profiles";
/// Hint footer text.
const HINT: &str = "j/k move  \u{23CE} select  esc back";
/// Empty-state message when there are no profiles.
const EMPTY: &str = "No profiles \u{2014} run ;n";

/// Render the profile picker into the given frame. The widget reads
/// `state.profiles` and `state.picker_index` but does not modify
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

    // Body rows. The list is scrolled so the highlighted row stays in
    // view: start at `picker_index.saturating_sub(BODY_ROWS - 1)`.
    if state.profiles.is_empty() {
        write_str_colored(frame, BODY_START, 0, EMPTY, dim, bg);
    } else {
        let start = state.picker_index.saturating_sub(BODY_ROWS - 1);
        for (i, idx) in (start..state.profiles.len()).take(BODY_ROWS).enumerate() {
            let row = BODY_START + i;
            if row >= HINT_ROW {
                break;
            }
            let profile = &state.profiles[idx];
            let color = if idx == state.picker_index { ok } else { fg };
            let line = format!("{}  {}", profile.label, profile.host);
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
    use crate::app::ProfileSummary;

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn two_profiles() -> Vec<ProfileSummary> {
        vec![
            ProfileSummary {
                id: "p1".into(),
                label: "default".into(),
                host: "aiserver-1".into(),
            },
            ProfileSummary {
                id: "p2".into(),
                label: "home".into(),
                host: "pc-ollama".into(),
            },
        ]
    }

    #[test]
    fn title_renders_in_accent_color() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState {
            profiles: two_profiles(),
            ..Default::default()
        };
        render(&mut f, &state, &theme);
        // Row 0: "Profiles" in accent.
        assert_eq!(f.cells[0][0].glyph, b'P');
        assert_eq!(f.cells[0][0].fg, theme.palette.accent.0);
    }

    #[test]
    fn both_profile_labels_visible() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState {
            profiles: two_profiles(),
            ..Default::default()
        };
        render(&mut f, &state, &theme);
        // Row 1: first label "default".
        assert_eq!(f.cells[1][0].glyph, b'd');
        // Row 2: second label "home".
        assert_eq!(f.cells[2][0].glyph, b'h');
    }

    #[test]
    fn empty_profiles_shows_empty_message() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState::default();
        render(&mut f, &state, &theme);
        // Row 1 starts with 'N' from "No profiles".
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
