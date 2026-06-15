//! Pure-data framebuffer. `Cell` and `Frame` contain no I/O.

use crate::layout::{COLS, ROWS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Index into the glyph atlas (a printable ASCII byte, 32..=127).
    pub glyph: u8,
    /// Foreground color in RGB565.
    pub fg: u16,
    /// Background color in RGB565.
    pub bg: u16,
    /// Attribute bitfield: bit 0 = bold, bit 1 = dim, bit 2 = inverse.
    pub attrs: u8,
}

impl Cell {
    /// A blank cell: space glyph, white on black, no attributes.
    pub const fn empty() -> Self {
        Self {
            glyph: b' ',
            fg: 0xFFFF,
            bg: 0x0000,
            attrs: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub cells: [[Cell; COLS]; ROWS],
}

impl Frame {
    /// Build a frame filled with the given background color and a space glyph
    /// in every cell.
    pub fn new_solid(bg: u16) -> Self {
        Self {
            cells: [[Cell {
                glyph: b' ',
                fg: 0xFFFF,
                bg,
                attrs: 0,
            }; COLS]; ROWS],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_empty_is_space_on_white_black() {
        let c = Cell::empty();
        assert_eq!(c.glyph, b' ');
        assert_eq!(c.fg, 0xFFFF);
        assert_eq!(c.bg, 0x0000);
        assert_eq!(c.attrs, 0);
    }

    #[test]
    fn frame_dimensions_are_40x16() {
        let f = Frame::new_solid(0);
        assert_eq!(f.cells.len(), ROWS);
        for row in &f.cells {
            assert_eq!(row.len(), COLS);
        }
    }

    #[test]
    fn frame_new_solid_sets_bg() {
        let f = Frame::new_solid(0x1234);
        for row in &f.cells {
            for c in row {
                assert_eq!(c.bg, 0x1234);
                assert_eq!(c.glyph, b' ');
            }
        }
    }
}
