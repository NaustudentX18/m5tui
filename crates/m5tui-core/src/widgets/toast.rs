//! Toast overlay for the M1 cockpit shell.
//!
//! A toast is a one-line message that appears on the last row of the
//! frame (row=15) when `state.toast` is `Some(_)`. It runs on top of
//! whatever the underlying widget drew (cockpit, palette, help). The
//! widget layer always calls `toast::render` last in `render::render`,
//! so the toast is the topmost layer.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// Draw the toast onto the last row of the frame. If `state.toast` is
/// `None`, this is a no-op. M2: the toast uses the theme's `warn` color
/// for the foreground and `bg` for the background.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let toast = match &state.toast {
        Some(t) => t,
        None => return,
    };
    let fg = theme.palette.warn.0;
    let bg = theme.palette.bg.0;
    let row = ROWS - 1;
    for col in 0..COLS {
        frame.cells[row][col] = Cell {
            glyph: b' ',
            fg,
            bg,
            attrs: 0,
        };
    }
    let max = COLS - 2;
    let truncated: String = toast.text.chars().take(max).collect();
    let bytes = truncated.as_bytes();
    let start_col = 1;
    for (i, b) in bytes.iter().enumerate() {
        let c = start_col + i;
        if c >= COLS - 1 {
            break;
        }
        frame.cells[row][c] = Cell {
            glyph: *b,
            fg,
            bg,
            attrs: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Toast;

    fn toast_state(text: &str) -> AppState {
        AppState {
            toast: Some(Toast {
                text: text.into(),
                until_tick: 100,
            }),
            ..AppState::default()
        }
    }

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    #[test]
    fn toast_renders_text_on_last_row() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &toast_state("Battery low"), &theme);
        let row = ROWS - 1;
        assert_eq!(f.cells[row][1].glyph, b'B');
        assert_eq!(f.cells[row][1].fg, theme.palette.warn.0);
        assert_eq!(f.cells[row][2].glyph, b'a');
        assert_eq!(f.cells[row][11].glyph, b'w');
    }

    #[test]
    fn toast_no_op_when_none() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &AppState::default(), &theme);
        let row = ROWS - 1;
        // A no-op must leave the frame untouched. Frame::new_solid fills
        // cells with the space glyph and the default white foreground.
        assert_eq!(f.cells[row][0].glyph, b' ');
        assert_eq!(f.cells[row][0].fg, 0xFFFF);
        assert_eq!(f.cells[row][0].bg, theme.palette.bg.0);
    }

    #[test]
    fn toast_truncates_long_text() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let long = "x".repeat(100);
        render(&mut f, &toast_state(&long), &theme);
        let row = ROWS - 1;
        for col in 0..COLS - 2 {
            assert_eq!(f.cells[row][col].fg, theme.palette.warn.0);
        }
        assert_eq!(f.cells[row][COLS - 1].glyph, b' ');
    }
}
