//! VU meter overlay for the cockpit bottom bar.
//!
//! The voice crate owns the actual audio capture; this module is a
//! tested renderer that turns a `0..=1` level into a 10-cell bar with
//! a left-edge `▏`-style mark. The widget is designed to drop into
//! row 0 (the top bar) or row 13 (the hint bar) without disturbing
//! the rest of the cockpit layout.
//!
//! The level is clamped to `0.0..=1.0`. The bar uses the theme's
//! `ok` colour up to 0.6, the theme's `accent` up to 0.85, and the
//! theme's `err` colour for the peak.

use crate::framebuffer::{Cell, Frame};
use m5tui_themes::Theme;

/// Build a 10-cell VU bar for the given `level` (clamped to 0..=1).
/// The returned string is 10 ASCII cells, suitable for direct copy
/// into the cockpit bottom bar. `peak` is optional — when `Some` and
/// larger than the current `level`, a single '^' marker is appended.
pub fn bar_string(level: f32, peak: Option<f32>) -> String {
    let clamped = level.clamp(0.0, 1.0);
    let filled = (clamped * 10.0).round() as usize;
    let mut s: String = "#".repeat(filled);
    while s.len() < 10 {
        s.push('-');
    }
    if let Some(p) = peak {
        if p > clamped {
            s.push('^');
        }
    }
    s
}

/// Draw a VU meter into the given frame at the specified (row, col).
/// The cell colours come from the theme. The bar is exactly 10 cells
/// wide and never wraps. An optional `peak` marker is drawn at col
/// +10 if the peak exceeds the current level.
pub fn draw(
    frame: &mut Frame,
    row: usize,
    col: usize,
    level: f32,
    peak: Option<f32>,
    theme: &Theme,
) {
    let bar = bar_string(level, peak);
    let bytes = bar.as_bytes();
    let ok = theme.palette.ok.0;
    let err = theme.palette.err.0;
    let dim = theme.palette.dim.0;
    for (i, &b) in bytes.iter().take(10).enumerate() {
        let cell_level = (i as f32) / 10.0;
        let fg = if cell_level > 0.85 {
            err
        } else if cell_level > 0.60 {
            theme.palette.accent.0
        } else if b == b'#' {
            ok
        } else {
            dim
        };
        frame.cells[row][col + i] = Cell {
            glyph: b,
            fg,
            bg: theme.palette.bg.0,
            attrs: 0,
        };
    }
    if bytes.len() > 10 {
        let i = 10;
        frame.cells[row][col + i] = Cell {
            glyph: b'^',
            fg: err,
            bg: theme.palette.bg.0,
            attrs: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use m5tui_themes;

    fn theme() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("missing coldwire theme"))
    }

    #[test]
    fn bar_string_silent_is_all_dashes() {
        assert_eq!(bar_string(0.0, None), "----------");
    }

    #[test]
    fn bar_string_full_is_all_hashes() {
        assert_eq!(bar_string(1.0, None), "##########");
    }

    #[test]
    fn bar_string_half_is_five_five() {
        assert_eq!(bar_string(0.5, None), "#####-----");
    }

    #[test]
    fn bar_string_clamps_negative() {
        assert_eq!(bar_string(-1.0, None), "----------");
    }

    #[test]
    fn bar_string_clamps_above_one() {
        assert_eq!(bar_string(2.0, None), "##########");
    }

    #[test]
    fn bar_string_appends_peak_marker() {
        assert_eq!(bar_string(0.4, Some(0.8)), "####------^");
    }

    #[test]
    fn bar_string_peak_marker_only_when_strictly_above() {
        // peak == level -> no marker.
        assert_eq!(bar_string(0.5, Some(0.5)), "#####-----");
    }

    #[test]
    fn draw_writes_ten_cells() {
        let mut frame = Frame::new_solid(0);
        draw(&mut frame, 0, 0, 0.7, None, &theme());
        for col in 0..10 {
            assert_ne!(frame.cells[0][col].glyph, b' ', "cell {col} was not drawn");
        }
    }

    #[test]
    fn draw_peak_marker_overflows_at_correct_col() {
        let mut frame = Frame::new_solid(0);
        draw(&mut frame, 0, 0, 0.3, Some(0.9), &theme());
        assert_eq!(frame.cells[0][10].glyph, b'^');
    }
}
