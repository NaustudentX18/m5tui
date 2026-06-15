//! Sim backend: convert a `Frame` into a 240x135 RGBA8888 byte buffer.
//!
//! The 40x16 cell grid is 240x128 pixels; the bottom 7 rows of the 240x135
//! physical screen are background. Output is pure-data and deterministic:
//! the same `Frame` always produces the same bytes. Foreground pixels get
//! `alpha = 0xFF`, background pixels get `alpha = 0x00`.

use crate::framebuffer::{Cell, Frame};
use crate::layout::{CELL_H, CELL_W, FB_H, FB_W};
use crate::render::glyph_cols;

/// Bytes per pixel in the output buffer.
const BYTES_PER_PIXEL: usize = 4;
/// Total bytes in the output buffer: 240 * 135 * 4 = 129600.
pub const RGBA_LEN: usize = FB_W * FB_H * BYTES_PER_PIXEL;

/// Expand an RGB565 color to 8-bit-per-channel RGB.
#[inline]
const fn rgb565_to_rgb888(c: u16) -> (u8, u8, u8) {
    let r = ((c >> 11) & 0x1F) as u8;
    let g = ((c >> 5) & 0x3F) as u8;
    let b = (c & 0x1F) as u8;
    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 2) | (g >> 4);
    let b8 = (b << 3) | (b >> 2);
    (r8, g8, b8)
}

/// Convert a `Frame` to a 240x135 RGBA8888 byte buffer. The buffer length is
/// always `RGBA_LEN` (129600).
pub fn render_to_rgba(frame: &Frame) -> Vec<u8> {
    let mut buf = vec![0u8; RGBA_LEN];
    for (row, row_cells) in frame.cells.iter().enumerate() {
        for (col, cell) in row_cells.iter().enumerate() {
            blit_cell(&mut buf, row, col, cell);
        }
    }
    buf
}

#[inline]
fn blit_cell(buf: &mut [u8], row: usize, col: usize, cell: &Cell) {
    let cols = glyph_cols(cell.glyph);
    let px_x = col * CELL_W;
    let py_y = row * CELL_H;
    let (fr, fg, fb) = rgb565_to_rgb888(cell.fg);

    // Cells are 6 pixels wide and the atlas stores 6 columns per glyph.
    // (The 6th column is the padding byte = 0, so glyph pixels never bleed
    // into the next cell.) Take only the first CELL_W=6 columns; the 7th
    // (if any) is the padding byte and would be zero anyway.
    for (gx, &col_byte) in cols.iter().take(CELL_W).enumerate() {
        let px = px_x + gx;
        for gy in 0..CELL_H {
            let pixel_set = (col_byte >> gy) & 1 == 1;
            let idx = ((py_y + gy) * FB_W + px) * BYTES_PER_PIXEL;
            if pixel_set {
                buf[idx] = fr;
                buf[idx + 1] = fg;
                buf[idx + 2] = fb;
                buf[idx + 3] = 0xFF;
            } else {
                buf[idx] = 0;
                buf[idx + 1] = 0;
                buf[idx + 2] = 0;
                buf[idx + 3] = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Frame;

    #[test]
    fn empty_frame_is_all_zero_alpha() {
        let f = Frame::new_solid(0x0000);
        let buf = render_to_rgba(&f);
        assert_eq!(buf.len(), RGBA_LEN);
        for pixel in buf.chunks_exact(4) {
            assert_eq!(pixel[3], 0, "bg pixel alpha should be 0");
        }
    }

    #[test]
    fn output_buffer_is_full_size() {
        // 40 * 6 = 240 wide, physical screen 135 tall, 4 bytes/pixel = 129600.
        let f = Frame::new_solid(0);
        let buf = render_to_rgba(&f);
        assert_eq!(buf.len(), 240 * 135 * 4);
    }
}
