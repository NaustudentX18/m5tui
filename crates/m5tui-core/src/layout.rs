//! Grid geometry for the 40x16 cell framebuffer and the 240x135 physical screen.
//!
//! The M5 Cardputer display is 240x135 pixels. The 40x16 character grid uses
//! 6 px wide x 8 px tall cells, so it covers 240x128 pixels. The remaining 7
//! pixel rows at the bottom of the display are background - no glyph data.

pub const COLS: usize = 40;
pub const ROWS: usize = 16;
pub const CELL_W: usize = 6;
pub const CELL_H: usize = 8;
/// Pixel height of the cell grid: `ROWS * CELL_H` = 128.
pub const GRID_H: usize = ROWS * CELL_H;
/// Pixel width of the cell grid and the physical screen: `COLS * CELL_W` = 240.
pub const FB_W: usize = COLS * CELL_W;
/// Physical screen height in pixels: 135 (the M5 Cardputer LCD is 240x135).
pub const FB_H: usize = 135;

/// X range for a cell column: (start_x_exclusive, end_x_exclusive).
#[inline]
pub const fn cell_x(col: usize) -> (usize, usize) {
    (col * CELL_W, (col + 1) * CELL_W)
}

/// Y range for a cell row: (start_y, end_y_exclusive).
#[inline]
pub const fn cell_y(row: usize) -> (usize, usize) {
    (row * CELL_H, (row + 1) * CELL_H)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_grid_is_40x16() {
        assert_eq!(COLS, 40);
        assert_eq!(ROWS, 16);
    }

    #[test]
    fn cell_size_is_6x8() {
        assert_eq!(CELL_W, 6);
        assert_eq!(CELL_H, 8);
    }

    #[test]
    fn physical_screen_is_240x135() {
        // The M5 Cardputer LCD is 240x135. The 40x16 cell grid only covers
        // 240x128; the remaining 7 rows are background.
        assert_eq!(FB_W, 240);
        assert_eq!(FB_H, 135);
        assert_eq!(GRID_H, 128);
    }

    #[test]
    fn cell_x_returns_correct_range() {
        assert_eq!(cell_x(0), (0, 6));
        assert_eq!(cell_x(1), (6, 12));
        assert_eq!(cell_x(13), (78, 84));
        assert_eq!(cell_x(39), (234, 240));
    }

    #[test]
    fn cell_y_returns_correct_range() {
        assert_eq!(cell_y(0), (0, 8));
        assert_eq!(cell_y(8), (64, 72));
        assert_eq!(cell_y(15), (120, 128));
    }
}
