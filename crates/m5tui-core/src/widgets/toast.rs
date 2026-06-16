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
use crate::palette;

/// Draw the toast onto the last row of the frame. If `state.toast` is
/// `None`, this is a no-op.
pub fn render(frame: &mut Frame, state: &AppState) {
    let toast = match &state.toast {
        Some(t) => t,
        None => return,
    };
    let row = ROWS - 1;
    for col in 0..COLS {
        frame.cells[row][col] = Cell {
            glyph: b' ',
            fg: palette::YELLOW,
            bg: palette::BG,
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
            fg: palette::YELLOW,
            bg: palette::BG,
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

    #[test]
    fn toast_renders_text_on_last_row() {
        let mut f = Frame::new_solid(palette::BG);
        render(&mut f, &toast_state("Battery low"));
        let row = ROWS - 1;
        assert_eq!(f.cells[row][1].glyph, b'B');
        assert_eq!(f.cells[row][1].fg, palette::YELLOW);
        assert_eq!(f.cells[row][2].glyph, b'a');
        assert_eq!(f.cells[row][11].glyph, b'w');
    }

    #[test]
    fn toast_no_op_when_none() {
        let mut f = Frame::new_solid(palette::BG);
        render(&mut f, &AppState::default());
        let row = ROWS - 1;
        assert_eq!(f.cells[row][0].glyph, b' ');
        assert_eq!(f.cells[row][0].fg, palette::FG);
    }

    #[test]
    fn toast_truncates_long_text() {
        let mut f = Frame::new_solid(palette::BG);
        let long = "x".repeat(100);
        render(&mut f, &toast_state(&long));
        let row = ROWS - 1;
        for col in 0..COLS - 2 {
            assert_eq!(f.cells[row][col].fg, palette::YELLOW);
        }
        assert_eq!(f.cells[row][COLS - 1].glyph, b' ');
    }
}
