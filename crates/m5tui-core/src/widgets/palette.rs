//! The M1 command palette modal.
//!
//! Layout (40x16):
//!
//! ```text
//!   0:  COMMAND PALETTE
//!   1:  search: {query}_
//!   2:  c  connect        Open profile connect picker
//!   3:  d  disconnect     Drop the current SSH session
//!
//!  ...
//!  13:  q  quit           Exit m5Tui
//!  14:  enter cast | esc close
//!  15:  (continuation of hint, optional)
//! ```
//!
//! The whole modal sits on `palette::SEL_BG`. The selected row uses
//! `SEL_FG` on `SEL_BG` (which we render as the inverse of the row's
//! normal colors — i.e. the row's text becomes SEL_BG-on-SEL_FG? No: we
//! keep the text in FG but invert the selected row's background to
//! SEL_BG and the foreground stays bright). The non-selected rows use
//! `FG` on `SEL_BG`.
//!
//! `palette::filter` is used with the live `palette_query`. An empty
//! query returns no matches from `fuzzy_score`, so the palette detects
//! that and shows the original `BUILTINS` in order.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use crate::palette::{filter, BUILTINS};
use m5tui_themes::Theme;

/// Draw the command palette into the given frame. Reads `state.palette_query`
/// and `state.palette_selected`. Idempotent and pure. M2: all colors come
/// from the theme; the modal sits on `theme.palette.sel_bg` and the
/// selected row uses `theme.palette.sel_fg` on `theme.palette.accent`.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let accent = theme.palette.accent.0;
    let sel_bg = theme.palette.sel_bg.0;
    let sel_fg = theme.palette.sel_fg.0;
    // Fill the whole frame with the theme's selection background.
    for row in 0..ROWS {
        for col in 0..COLS {
            frame.cells[row][col] = Cell {
                glyph: b' ',
                fg,
                bg: sel_bg,
                attrs: 0,
            };
        }
    }

    write_str_colored(frame, 0, 0, "COMMAND PALETTE", accent, sel_bg);
    write_str_colored(
        frame,
        1,
        0,
        &format!("search: {}_", state.palette_query),
        fg,
        sel_bg,
    );

    let entries = palette_entries(&state.palette_query);
    let selected = state.palette_selected.min(entries.len().saturating_sub(1));
    for (i, (cmd_idx, _score)) in entries.iter().take(12).enumerate() {
        let row = 2 + i;
        if row >= 14 {
            break;
        }
        let cmd = &BUILTINS[*cmd_idx];
        let is_selected = i == selected;
        let (row_fg, row_bg) = if is_selected {
            (sel_fg, accent)
        } else {
            (fg, sel_bg)
        };
        // "{key}  {name:14.14} {desc}" — 2-char key + 2 spaces + name + space + desc.
        let line = format!("{}  {:<14} {}", cmd.key, cmd.name, cmd.desc,);
        // The first 3 chars are the key + 2 spaces; render the key in accent
        // so it stands out. The rest of the line goes in the row's fg/bg.
        if !line.is_empty() {
            frame.cells[row][0] = Cell {
                glyph: line.as_bytes()[0],
                fg: accent,
                bg: row_bg,
                attrs: 0,
            };
        }
        write_str_colored(
            frame,
            row,
            1,
            &line[1..min_usize(line.len(), 38)],
            row_fg,
            row_bg,
        );
    }

    let hint = "enter cast | esc close";
    write_str_colored(frame, 14, 0, hint, fg, sel_bg);
}

/// Run `palette::filter` over the builtin list, falling back to the
/// original 12 in order when the query is empty (or matches nothing).
fn palette_entries(query: &str) -> Vec<(usize, i32)> {
    if query.is_empty() {
        return (0..BUILTINS.len()).map(|i| (i, 0)).collect();
    }
    let hits = filter(query, &BUILTINS);
    if hits.is_empty() {
        (0..BUILTINS.len()).map(|i| (i, 0)).collect()
    } else {
        hits
    }
}

#[inline]
fn min_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

fn write_str_colored(frame: &mut Frame, row: usize, col: usize, s: &str, fg: u16, bg: u16) {
    let row = if row < ROWS { row } else { return };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Mode;
    use crate::event::Focus;

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn palette_state(query: &str) -> AppState {
        AppState {
            mode: Mode::Palette,
            focus: Focus::Prompt,
            palette_query: query.into(),
            palette_selected: 0,
            ..AppState::default()
        }
    }

    #[test]
    fn palette_empty_query_shows_all_12_builtins() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &palette_state(""), &theme);
        // The first command in BUILTINS is `connect`. Its name should be
        // visible at (row=2, col=3) — the `c` key is at col 0, two spaces
        // of padding, then the 14-char name field. The first char of
        // `connect` is at col 3.
        assert_eq!(f.cells[2][3].glyph, b'c');
        assert_eq!(f.cells[2][4].glyph, b'o');
        // The 12th command (`quit`) should be visible at row 13.
        let q_row = 2 + 11;
        assert_eq!(f.cells[q_row][3].glyph, b'q');
    }

    #[test]
    fn palette_query_c_promotes_connect() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &palette_state("c"), &theme);
        // `connect` matches `c` as a prefix and scores higher than
        // subsequence matches. It should appear at the top of the
        // filtered list at row 2.
        assert_eq!(f.cells[2][3].glyph, b'c');
        assert_eq!(f.cells[2][4].glyph, b'o');
    }

    #[test]
    fn palette_renders_title_in_accent() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &palette_state(""), &theme);
        // Title at row 0 is `COMMAND PALETTE` in the theme accent on sel_bg.
        assert_eq!(f.cells[0][0].glyph, b'C');
        assert_eq!(f.cells[0][0].fg, theme.palette.accent.0);
        assert_eq!(f.cells[0][0].bg, theme.palette.sel_bg.0);
    }

    #[test]
    fn palette_renders_search_line_with_cursor() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &palette_state("co"), &theme);
        // The search line at row 1 reads `search: co_` with a cursor
        // glyph at the end of the query.
        let prefix = "search: co";
        for (i, b) in prefix.bytes().enumerate() {
            assert_eq!(f.cells[1][i].glyph, b);
        }
        // Cursor at col 10 (`search: co` is 10 chars).
        assert_eq!(f.cells[1][10].glyph, b'_');
    }

    #[test]
    fn palette_marks_selected_row_with_inverse_colors() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = AppState {
            palette_selected: 2,
            ..palette_state("")
        };
        render(&mut f, &s, &theme);
        // Row 4 is the 3rd entry (index 2) and is selected. Its
        // background is the theme accent and the text is sel_fg.
        assert_eq!(f.cells[4][0].bg, theme.palette.accent.0);
        // The 2nd row (index 0, `connect`) is not selected: bg = sel_bg.
        assert_eq!(f.cells[2][1].bg, theme.palette.sel_bg.0);
    }
}
