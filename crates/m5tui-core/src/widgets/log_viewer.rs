//! Log viewer overlay — scrollable, follow-tail log buffer view.
//!
//! Layout (40 cols × 16 rows):
//!   row  0: blank
//!   row  1: title `Log` + count `(N/M)` in accent
//!   row  2: mode indicator `FOLLOW` (ok color) or `SCROLL` (dim color)
//!   row  3: blank
//!   rows 4..=13: 10 log lines from `state.log_buffer`, clipped to 40
//!           columns and truncated with `…` at column 39
//!   row 14: blank
//!   row 15: hint `j/k scroll · g/G top/bottom · f follow · Esc close`
//!
//! When `tail_mode` is on, the last 10 buffered lines are shown. When
//! the user has scrolled back, the 10 lines ending at
//! `lines.len() - 1 - scroll_offset` are shown, padded with blank rows
//! above as needed.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// Number of body rows available for log lines.
const BODY_ROWS: usize = 10;
/// Row index where the first body line lands.
const BODY_START: usize = 4;

/// Draw the log viewer overlay into the given frame.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    let ok = theme.palette.ok.0;
    let dim = theme.palette.dim.0;

    // Solid background.
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

    // Title + count.
    let count = state.log_buffer.len();
    let max = state.log_buffer.max_lines;
    let title = format!("Log ({count}/{max})");
    write_str_colored(frame, 1, 0, &title, accent, bg);

    // Mode indicator.
    let (mode_label, mode_color) = if state.log_buffer.tail_mode {
        ("FOLLOW", ok)
    } else {
        ("SCROLL", dim)
    };
    write_str_colored(frame, 2, 0, mode_label, mode_color, bg);

    // Body lines. Compute the visible window.
    let lines: Vec<&str> = state.log_buffer.lines().collect();
    let visible = visible_window(
        &lines,
        state.log_buffer.scroll_offset,
        state.log_buffer.tail_mode,
    );
    for (i, line) in visible.iter().enumerate() {
        let row = BODY_START + i;
        if row >= ROWS - 2 {
            break;
        }
        write_line(frame, row, line, fg, bg);
    }

    // Hint footer.
    let hint = "j/k scroll · g/G top/bottom · f follow · Esc close";
    write_str_colored(frame, ROWS - 1, 0, hint, dim, bg);
}

/// Pick the lines to show in the body area. Returns a `Vec<&str>` of
/// length `<= BODY_ROWS`, oldest-first.
fn visible_window<'a>(lines: &[&'a str], scroll_offset: usize, tail_mode: bool) -> Vec<&'a str> {
    if lines.is_empty() {
        return Vec::new();
    }
    let total = lines.len();
    let end = if tail_mode {
        total
    } else {
        total.saturating_sub(scroll_offset)
    };
    let start = end.saturating_sub(BODY_ROWS);
    lines[start..end].to_vec()
}

/// Draw a single line of log text into `row`, clipping at `COLS` and
/// truncating with `…` when the text overflows.
fn write_line(frame: &mut Frame, row: usize, line: &str, fg: u16, bg: u16) {
    let max_cols = COLS;
    let bytes = line.as_bytes();
    if bytes.len() < max_cols {
        write_str_colored(frame, row, 0, line, fg, bg);
        return;
    }
    // Truncate to (max_cols - 1) bytes and replace the last visible
    // column with `…`.
    let take = max_cols - 1;
    write_str_colored(frame, row, 0, &line[..take], fg, bg);
    frame.cells[row][max_cols - 1] = Cell {
        glyph: 0xC3, // … in ISO-8859-1; single-byte fallback.
        fg,
        bg,
        attrs: 0,
    };
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, LogBuffer, Mode};

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn state_with(lines: Vec<String>, scroll_offset: usize, tail_mode: bool) -> AppState {
        let mut buf = LogBuffer::new(200);
        for l in lines {
            buf.push(l);
        }
        buf.scroll_offset = scroll_offset;
        buf.tail_mode = tail_mode;
        AppState {
            mode: Mode::LogViewer,
            log_buffer: buf,
            ..AppState::default()
        }
    }

    #[test]
    fn empty_buffer_renders_title_only() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &state_with(vec![], 0, true), &theme);
        // Title cell at row 1 col 0 is 'L' in accent.
        assert_eq!(f.cells[1][0].glyph, b'L');
        assert_eq!(f.cells[1][0].fg, theme.palette.accent.0);
        // Follow mode indicator at row 2 col 0.
        assert_eq!(f.cells[2][0].glyph, b'F');
        assert_eq!(f.cells[2][0].fg, theme.palette.ok.0);
        // Body rows 4..=13 are blank.
        for row in 4..=13 {
            assert_eq!(f.cells[row][0].glyph, b' ');
        }
        // Hint row 15 starts with 'j'.
        assert_eq!(f.cells[ROWS - 1][0].glyph, b'j');
        assert_eq!(f.cells[ROWS - 1][0].fg, theme.palette.dim.0);
    }

    #[test]
    fn single_line_renders_into_body() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &state_with(vec!["hello".into()], 0, true), &theme);
        // Title shows (1/200).
        let title_row: Vec<u8> = (0..COLS).map(|c| f.cells[1][c].glyph).collect();
        let title = String::from_utf8_lossy(&title_row);
        assert!(title.contains("(1/200)"), "title was: {title}");
        // First body row at row 4 starts with 'h'.
        assert_eq!(f.cells[4][0].glyph, b'h');
        assert_eq!(f.cells[4][1].glyph, b'e');
    }

    #[test]
    fn full_buffer_overflow_drops_oldest_lines() {
        // Push 250 lines into a 200-line buffer; the first 50 are dropped.
        let mut buf = LogBuffer::new(200);
        for i in 0..250 {
            buf.push(format!("line {i:03}"));
        }
        assert_eq!(buf.len(), 200);
        let lines: Vec<&str> = buf.lines().collect();
        assert_eq!(lines.first().copied(), Some("line 050"));
        assert_eq!(lines.last().copied(), Some("line 249"));
    }

    #[test]
    fn scroll_up_increases_offset_and_disables_follow() {
        let mut buf = LogBuffer::new(8);
        for i in 0..5 {
            buf.push(format!("l{i}"));
        }
        buf.scroll_up();
        assert_eq!(buf.scroll_offset, 1);
        assert!(!buf.tail_mode);
        buf.scroll_up();
        assert_eq!(buf.scroll_offset, 2);
        buf.scroll_up();
        buf.scroll_up();
        // Saturates at oldest line.
        assert_eq!(buf.scroll_offset, 4);
    }

    #[test]
    fn scroll_top_jumps_to_oldest() {
        let mut buf = LogBuffer::new(8);
        for i in 0..6 {
            buf.push(format!("l{i}"));
        }
        buf.scroll_top();
        assert_eq!(buf.scroll_offset, 5);
        assert!(!buf.tail_mode);
    }

    #[test]
    fn scroll_bottom_returns_to_tail() {
        let mut buf = LogBuffer::new(8);
        for i in 0..6 {
            buf.push(format!("l{i}"));
        }
        buf.scroll_top();
        buf.scroll_bottom();
        assert_eq!(buf.scroll_offset, 0);
        assert!(buf.tail_mode);
    }

    #[test]
    fn toggle_follow_flips_flag() {
        let mut buf = LogBuffer::new(8);
        for i in 0..3 {
            buf.push(format!("l{i}"));
        }
        assert!(buf.tail_mode);
        buf.toggle_follow();
        assert!(!buf.tail_mode);
        buf.toggle_follow();
        assert!(buf.tail_mode);
    }

    #[test]
    fn lines_longer_than_40_cols_truncate_with_marker() {
        let theme = coldwire();
        let long = "x".repeat(80);
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &state_with(vec![long], 0, true), &theme);
        // Last column of body row 4 is the truncation marker.
        assert_eq!(f.cells[4][COLS - 1].glyph, 0xC3);
        // Columns 0..=38 are filled with 'x'.
        for c in 0..COLS - 1 {
            assert_eq!(f.cells[4][c].glyph, b'x', "col {c} not 'x'");
        }
    }

    #[test]
    fn scrolled_window_shows_older_lines() {
        let theme = coldwire();
        // 20 lines; scroll_offset = 3, tail_mode=false → end = 17 → rows 7..17.
        let mut s = state_with((0..20).map(|i| format!("line {i:02}")).collect(), 3, false);
        s.mode = Mode::LogViewer;
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &s, &theme);
        // Row 4 should start with "line 07".
        let row: Vec<u8> = (0..COLS).map(|c| f.cells[4][c].glyph).collect();
        let text = String::from_utf8_lossy(&row);
        assert!(text.starts_with("line 07"), "first body row was: {text}");
        // Row 13 should be "line 16".
        let row: Vec<u8> = (0..COLS).map(|c| f.cells[13][c].glyph).collect();
        let text = String::from_utf8_lossy(&row);
        assert!(text.starts_with("line 16"), "last body row was: {text}");
    }

    #[test]
    fn scroll_mode_indicator_uses_dim_color() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &state_with(vec!["a".into()], 1, false), &theme);
        // Row 2 starts with 'S' (SCROLL) in dim color.
        assert_eq!(f.cells[2][0].glyph, b'S');
        assert_eq!(f.cells[2][0].fg, theme.palette.dim.0);
    }

    #[test]
    fn tail_mode_following_does_not_show_marker_when_short() {
        // Short lines (< 40 cols) should NOT trigger the truncation
        // marker even though we are in tail mode.
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &state_with(vec!["short".into()], 0, true), &theme);
        assert_eq!(f.cells[4][0].glyph, b's');
        // Last column should be blank (the rest of the row is filled
        // with spaces by the background pass).
        assert_eq!(f.cells[4][COLS - 1].glyph, b' ');
    }
}
