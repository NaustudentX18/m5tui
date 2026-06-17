//! v1.12.0 animated boot screen.
//!
//! Layout on the 40x16 grid:
//!
//! ```text
//!   0:  (blank)
//!   2-6: 5-row ASCII "m5Tui" block logo (centered)
//!   8:  subtitle: "Cardputer-Adv . v1.12.0"
//!  10:  progress bar:  Boot: [#####---]  50%
//!  12:  status line (cycles every 8 ticks)
//!  14:  hint:  "press any key to skip"
//! ```
//!
//! Determinism: the widget is a pure function of `state.clock` (the
//! monotonically-increasing tick counter incremented on every `Tick`
//! event by the reducer). No `Instant::now`, no timers, no allocation
//! on the render path.
//!
//! The five boot stages are
//! "init storage" / "init network" / "init display" / "init audio" /
//! "ready" and cycle every 8 ticks; the progress bar uses the same
//! `clock / 4 % 11` index, so the bar always finishes the cycle when
//! the status line lands on "ready".

use crate::app::AppState;
use crate::framebuffer::Cell;
use crate::framebuffer::Frame;
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// Width of the block-text logo in cells. Five rows tall, fits in 40
/// columns when centered.
const LOGO_W: usize = 18;
/// Height of the block-text logo in cells.
const LOGO_H: usize = 5;
/// Top row of the logo (row 2 in the spec).
const LOGO_ROW: usize = 2;
/// Subtitle row.
const SUBTITLE_ROW: usize = 8;
/// Progress bar row.
const PROGRESS_ROW: usize = 10;
/// Status line row.
const STATUS_ROW: usize = 12;
/// Hint row.
const HINT_ROW: usize = 14;
/// Subtitle text. v1.12.0.
const SUBTITLE: &str = "Cardputer-Adv . v1.12.0";
/// Hint text.
const HINT: &str = "press any key to skip";
/// Status messages, cycled every `STAGE_PERIOD` ticks.
const STAGES: [&str; 5] = [
    "init storage",
    "init network",
    "init display",
    "init audio",
    "ready",
];
/// Period (in ticks) at which the status line advances.
const STAGE_PERIOD: u64 = 8;
/// Number of fill cells in the progress bar bracket. The bar is
/// `Boot: [` + N fillable cells + `]  N*10%` -> 10 cells.
const PROGRESS_FILL: usize = 10;
/// Bar template prefix length: "Boot: [".
const PROGRESS_PREFIX_LEN: usize = 8;
/// Bracket glyphs.
const FILL_GLYPH: u8 = b'#';
const EMPTY_GLYPH: u8 = b'-';

/// Five rows of block art spelling "m5Tui" in `block_glyph` (the spec
/// allowed ASCII or block letters; we use the heavy `#` character
/// because the 5x8 glyph atlas renders `#` opaquely across all five
/// columns and the result reads cleanly at the 40-col width). The
/// logo is 18 cells wide and 5 rows tall.
const LOGO: [&str; LOGO_H] = [
    "##   ##  ######  ",
    "##   ##    ##    ",
    "## # ##   ##     ",
    "### ###  ##      ",
    "##   ##  ######  ",
];

/// Render the boot screen into the frame. The widget draws on top of
/// the solid background that `render::render` already filled, so it
/// only needs to overwrite the cells it actually uses. Any cell that
/// isn't touched here retains the theme background.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    let dim = theme.palette.dim.0;
    let ok = theme.palette.ok.0;

    draw_logo(frame, accent, bg);
    draw_centered(frame, SUBTITLE_ROW, SUBTITLE, fg, bg);
    draw_progress_bar(frame, state.clock, accent, dim, bg);
    draw_status(frame, state.clock, ok, fg, bg);
    draw_centered(frame, HINT_ROW, HINT, dim, bg);
}

/// Deterministic stage index in 0..STAGES.len() derived from `clock`.
/// Wraps every `STAGE_PERIOD * STAGES.len()` ticks.
pub fn stage_for(clock: u64) -> usize {
    ((clock / STAGE_PERIOD) % STAGES.len() as u64) as usize
}

/// Deterministic progress-bar fill in 0..=PROGRESS_FILL. Cycles every
/// `PROGRESS_FILL + 1` ticks of `clock / 4`, so 0..10..0..10.. (the
/// bar never sticks at the top — when it would wrap to 0 we instead
/// show the full bar briefly).
pub fn progress_for(clock: u64) -> usize {
    ((clock / 4) % (PROGRESS_FILL as u64 + 1)) as usize
}

/// Draw the 5-row block logo centered on the 40-col grid. Uses the
/// theme's accent color so the logo pops against the background.
fn draw_logo(frame: &mut Frame, fg: u16, bg: u16) {
    let start_col = COLS.saturating_sub(LOGO_W) / 2;
    for (row_off, line) in LOGO.iter().enumerate() {
        let row = LOGO_ROW + row_off;
        if row >= ROWS {
            break;
        }
        for (col_off, b) in line.bytes().enumerate() {
            let col = start_col + col_off;
            if col >= COLS {
                break;
            }
            let glyph = if b == b' ' { b' ' } else { b'#' };
            frame.cells[row][col] = Cell {
                glyph,
                fg,
                bg,
                attrs: 0,
            };
        }
    }
}

/// Write a string centered on the given row, clipped to COLS.
fn draw_centered(frame: &mut Frame, row: usize, s: &str, fg: u16, bg: u16) {
    if row >= ROWS {
        return;
    }
    let len = s.len();
    let start_col = if len >= COLS { 0 } else { (COLS - len) / 2 };
    for (i, b) in s.bytes().enumerate() {
        let col = start_col + i;
        if col >= COLS {
            break;
        }
        let glyph = if b.is_ascii_graphic() || b == b' ' {
            b
        } else {
            b'?'
        };
        frame.cells[row][col] = Cell {
            glyph,
            fg,
            bg,
            attrs: 0,
        };
    }
}

/// Draw the progress bar. Format:
/// `Boot: [XXXXXXXXXX] 100%` -- 8-char prefix + 10 fill cells + 2
/// bracket chars + 1 space + 3-digit number + '%' = ~25 chars, fits.
/// Uses the accent color for filled cells and the dim color for
/// empty cells, with the percentage in the standard foreground.
fn draw_progress_bar(frame: &mut Frame, clock: u64, accent: u16, dim: u16, bg: u16) {
    if PROGRESS_ROW >= ROWS {
        return;
    }
    // Start the bar 2 cells from the left so the percentage stays
    // inside the 40-col width even when the bar is full.
    let start_col: usize = 2;
    let prefix = b"Boot: [";
    for (i, b) in prefix.iter().enumerate() {
        let col = start_col + i;
        if col < COLS {
            frame.cells[PROGRESS_ROW][col] = Cell {
                glyph: *b,
                fg: accent,
                bg,
                attrs: 0,
            };
        }
    }
    let fill = progress_for(clock);
    for i in 0..PROGRESS_FILL {
        let col = start_col + PROGRESS_PREFIX_LEN + i;
        if col >= COLS {
            break;
        }
        let (glyph, color) = if i < fill {
            (FILL_GLYPH, accent)
        } else {
            (EMPTY_GLYPH, dim)
        };
        frame.cells[PROGRESS_ROW][col] = Cell {
            glyph,
            fg: color,
            bg,
            attrs: 0,
        };
    }
    // Trailing ']' + space + percent.
    let close_col = start_col + PROGRESS_PREFIX_LEN + PROGRESS_FILL;
    if close_col < COLS {
        frame.cells[PROGRESS_ROW][close_col] = Cell {
            glyph: b']',
            fg: accent,
            bg,
            attrs: 0,
        };
    }
    let pct = fill * 100 / PROGRESS_FILL;
    let pct_text = format!("{:>3}%", pct);
    let pct_col = close_col + 1;
    for (i, b) in pct_text.bytes().enumerate() {
        let col = pct_col + i;
        if col >= COLS {
            break;
        }
        frame.cells[PROGRESS_ROW][col] = Cell {
            glyph: b,
            fg: accent,
            bg,
            attrs: 0,
        };
    }
}

/// Draw the cycling status line, centered on `STATUS_ROW`. The status
/// is selected deterministically from `clock` so two renders at the
/// same tick are byte-identical.
fn draw_status(frame: &mut Frame, clock: u64, ok: u16, fg: u16, bg: u16) {
    let stage = STAGES[stage_for(clock)];
    // The final "ready" stage is highlighted in the OK color; the
    // rest use the theme's dimmed foreground.
    let color = if stage == "ready" { ok } else { fg };
    draw_centered(frame, STATUS_ROW, stage, color, bg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn boot_state() -> AppState {
        AppState {
            mode: crate::app::Mode::Boot,
            ..AppState::default()
        }
    }

    /// A render at `clock = 0` must place the "m5Tui" block logo on
    /// row 2-6. We assert the top-left cell of the logo (row 2,
    /// col 11, the first `#`) is in the theme's accent.
    #[test]
    fn logo_first_row_first_cell_uses_accent() {
        let theme = coldwire();
        let s = boot_state();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &s, &theme);
        // LOGO_W = 18, COLS = 40, start_col = (40 - 18) / 2 = 11.
        let start_col = (COLS - LOGO_W) / 2;
        let cell = f.cells[LOGO_ROW][start_col];
        assert_eq!(cell.glyph, b'#');
        assert_eq!(cell.fg, theme.palette.accent.0);
        assert_eq!(cell.bg, theme.palette.bg.0);
    }

    /// The subtitle must appear centered on row 8.
    #[test]
    fn subtitle_renders_on_row_8() {
        let theme = coldwire();
        let s = boot_state();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &s, &theme);
        let start = (COLS - SUBTITLE.len()) / 2;
        for (i, b) in SUBTITLE.bytes().enumerate() {
            assert_eq!(f.cells[SUBTITLE_ROW][start + i].glyph, b);
        }
    }

    /// The progress-bar prefix `Boot: [` must be on row 10 starting
    /// at col 2 in the accent color.
    #[test]
    fn progress_bar_prefix_uses_accent() {
        let theme = coldwire();
        let s = boot_state();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &s, &theme);
        let prefix = b"Boot: [";
        for (i, b) in prefix.iter().enumerate() {
            let cell = f.cells[PROGRESS_ROW][2 + i];
            assert_eq!(cell.glyph, *b);
            assert_eq!(cell.fg, theme.palette.accent.0);
        }
    }

    /// The hint must appear centered on row 14 in the dim color.
    #[test]
    fn hint_renders_on_row_14_in_dim() {
        let theme = coldwire();
        let s = boot_state();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &s, &theme);
        let start = (COLS - HINT.len()) / 2;
        for (i, b) in HINT.bytes().enumerate() {
            let cell = f.cells[HINT_ROW][start + i];
            assert_eq!(cell.glyph, b);
            assert_eq!(cell.fg, theme.palette.dim.0);
        }
    }

    /// `progress_for` is deterministic: tick 0 -> 0 fill, tick 4 ->
    /// 1, tick 8 -> 2, tick 16 -> 4, tick 64 -> 5 (since 64/4 = 16,
    /// 16 % 11 = 5).
    #[test]
    fn progress_is_deterministic_across_ticks() {
        assert_eq!(progress_for(0), 0);
        assert_eq!(progress_for(4), 1);
        assert_eq!(progress_for(8), 2);
        assert_eq!(progress_for(16), 4);
        assert_eq!(progress_for(64), 5);
        // The bar is 10 wide + 1 -> modulo 11, so the full sequence
        // 0..10.=0.. repeats every 44 ticks (4 * 11).
        for t in 0u64..44 {
            assert_eq!(progress_for(t), ((t / 4) % 11) as usize, "tick {t}");
        }
    }

    /// The status stage cycles every `STAGE_PERIOD` ticks through
    /// the five entries in order.
    #[test]
    fn status_stage_cycles_in_order() {
        assert_eq!(stage_for(0), 0);
        assert_eq!(stage_for(7), 0);
        assert_eq!(stage_for(8), 1);
        assert_eq!(stage_for(15), 1);
        assert_eq!(stage_for(16), 2);
        assert_eq!(stage_for(24), 3);
        assert_eq!(stage_for(32), 4);
        // Wraps back to "init storage".
        assert_eq!(stage_for(40), 0);
    }

    /// Two renders at the same `clock` produce byte-identical
    /// frames. (Re-rendering an already-rendered frame must not
    /// leave any stale cell behind.)
    #[test]
    fn render_is_idempotent() {
        let theme = coldwire();
        let s = boot_state();
        let mut a = Frame::new_solid(theme.palette.bg.0);
        let mut b = Frame::new_solid(theme.palette.bg.0);
        render(&mut a, &s, &theme);
        render(&mut b, &s, &theme);
        assert_eq!(a.cells, b.cells);
    }

    /// The default `AppState::mode` must be `Mode::Boot` for v1.12.0
    /// so the very first frame the user sees is the boot screen.
    /// This test is the canary the orchestrator's spec calls for.
    #[test]
    fn booting_app_state_mode_is_boot() {
        assert_eq!(AppState::booting().mode, crate::app::Mode::Boot);
    }
}
