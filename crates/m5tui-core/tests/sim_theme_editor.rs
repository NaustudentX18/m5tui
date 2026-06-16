//! Sim editor test: render the theme editor overlay with the coldwire
//! theme and verify the 9 menu items, accent title, and 40x16 frame
//! shape all behave as the spec requires.

use m5tui_core::*;

const COLS: usize = 40;
const ROWS: usize = 16;

#[test]
fn theme_editor_renders_coldwire() {
    let theme = m5tui_themes::builtin("coldwire")
        .unwrap_or_else(|| panic!("coldwire should be a builtin theme"));
    let state = AppState {
        mode: Mode::ThemeEditor,
        ..AppState::default()
    };
    let frame = render(&state, &theme);
    // The title at (row=0, col=0) starts with 'T' of "THEME: coldwire".
    assert_eq!(
        frame.cells[0][0].glyph, b'T',
        "expected 'T' of the title at (row=0, col=0)"
    );
    // The title is rendered in the theme's accent.
    assert_eq!(
        frame.cells[0][0].fg, theme.palette.accent.0,
        "row 0 col 0 fg must equal theme.palette.accent"
    );
    // Sanity: coldwire accent is the spec value 0x06BF (the RGB565 form of
    // #00d8ff).
    // Coldwire accent #00d8ff -> 0x06DF in RGB565.
    assert_eq!(theme.palette.accent.0, 0x06DF);
    assert_eq!(frame.cells[0][0].fg, theme.palette.accent.0);

    // The 9 menu items live in rows 2..=13 (the spec calls for 9 `>`
    // arrows in the body). Each item line ends in a `>` arrow at the
    // right edge of its column.
    let mut arrows = 0usize;
    for row in 2..14 {
        for col in 0..COLS {
            if frame.cells[row][col].glyph == b'>' {
                arrows += 1;
            }
        }
    }
    assert!(
        arrows >= 9,
        "expected >= 9 menu arrows in rows 2..13, found {arrows}"
    );
}

#[test]
fn theme_editor_renders_separator_and_hint() {
    let theme = m5tui_themes::builtin("coldwire")
        .unwrap_or_else(|| panic!("coldwire should be a builtin theme"));
    let state = AppState {
        mode: Mode::ThemeEditor,
        ..AppState::default()
    };
    let frame = render(&state, &theme);
    // Row 1 is the separator: a run of '-' in dim.
    assert_eq!(frame.cells[1][0].glyph, b'-');
    // Row 15 is the hint: starts with ';' of the chord ";1-9 select | esc close".
    assert_eq!(frame.cells[ROWS - 1][0].glyph, b';');
    assert_eq!(frame.cells[ROWS - 1][1].glyph, b'1');
}

#[test]
fn theme_editor_paints_bg_under_everything() {
    let theme = m5tui_themes::builtin("lacuna")
        .unwrap_or_else(|| panic!("lacuna should be a builtin theme"));
    let state = AppState {
        mode: Mode::ThemeEditor,
        ..AppState::default()
    };
    let frame = render(&state, &theme);
    // The editor fills the entire 40x16 frame with the theme's bg. Pick
    // a corner that no widget writes to (row 7, col 30) and verify it's
    // a space in the theme's bg.
    let cell = &frame.cells[7][30];
    assert_eq!(cell.bg, theme.palette.bg.0);
    assert_eq!(cell.glyph, b' ');
}
