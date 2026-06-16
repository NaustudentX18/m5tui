//! Sim golden tests: one per built-in theme.
//!
//! Each test boots the default `AppState`, loads the named theme via
//! `m5tui_themes::builtin(name).unwrap()`, renders the frame, and asserts:
//! - The RGBA buffer is exactly 240*135*4 = 129600 bytes.
//! - The cell at (row=0, col=0) holds the cockpit's top bar (server-status
//!   line) with `bg = theme.palette.bg.0`.
//! - The cell at (row=1, col=0) holds the AGENTS header with
//!   `fg = theme.palette.accent.0`.
//!
//! The cell-level assertions ride on the cockpit widget, which the
//! theme-agnostic layer renders before the new theme system. The contract
//! being verified here is that the cockpit's accent cells pick up the
//! theme's accent color slot, not a hardcoded `palette::ACCENT` constant.

use m5tui_core::*;

const RGBA_LEN: usize = 240 * 135 * 4;

fn frame_buf_for(name: &str) -> (Frame, Vec<u8>, m5tui_themes::Theme) {
    let state = AppState::default();
    let theme =
        m5tui_themes::builtin(name).unwrap_or_else(|| panic!("builtin({name}) returned None"));
    let frame = render(&state, &theme);
    let buf = sim::render_to_rgba(&frame);
    assert_eq!(
        buf.len(),
        RGBA_LEN,
        "expected 129600 bytes for 240x135 RGBA (theme={name})"
    );
    (frame, buf, theme)
}

macro_rules! theme_test {
    ($name:ident, $theme:literal) => {
        #[test]
        fn $name() {
            let (frame, _buf, theme) = frame_buf_for($theme);
            // Row 0, col 0 is the cockpit top bar (the 'a' of "aiserver-1").
            // The cockpit renders the whole top bar in FG on BG, so the bg
            // must be the theme's background.
            let top_left = &frame.cells[0][0];
            assert_eq!(
                top_left.bg, theme.palette.bg.0,
                "row 0 col 0 bg must equal theme.palette.bg (theme={})",
                $theme
            );
            assert_eq!(
                top_left.glyph, b'a',
                "row 0 col 0 should be the 'a' of 'aiserver-1' (theme={})",
                $theme
            );
            // Row 1, col 0 is the AGENTS header in accent.
            let agents_hdr = &frame.cells[1][0];
            assert_eq!(
                agents_hdr.fg, theme.palette.accent.0,
                "row 1 col 0 fg must equal theme.palette.accent (theme={})",
                $theme
            );
            assert_eq!(
                agents_hdr.glyph, b'A',
                "row 1 col 0 should be the 'A' of 'AGENTS' (theme={})",
                $theme
            );
        }
    };
}

theme_test!(coldwire_golden, "coldwire");
theme_test!(phosphor_golden, "phosphor");
theme_test!(lacuna_golden, "lacuna");
theme_test!(magline_golden, "magline");
theme_test!(noctilux_golden, "noctilux");
theme_test!(ivoryroom_golden, "ivoryroom");
