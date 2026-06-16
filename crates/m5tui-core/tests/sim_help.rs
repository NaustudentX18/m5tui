//! Sim help tests: verify the help overlay renders the title in the
//! accent color, shows the `esc close` hint, and lists the binding
//! table. The widget lays the bindings out in two columns starting at
//! row 1; the bottom row is reserved for the hint.

use m5tui_core::*;

fn coldwire() -> m5tui_themes::Theme {
    m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
}

fn help_state() -> AppState {
    AppState {
        mode: Mode::Help,
        focus: Focus::Prompt,
        ..AppState::default()
    }
}

#[test]
fn help_renders_title() {
    let theme = coldwire();
    let frame = render(&help_state(), &theme);
    // Title at (row=0, col=0) begins with `m` (from "m5Tui v0.1.0 -- HOTKEYS").
    assert_eq!(
        frame.cells[0][0].glyph, b'm',
        "expected 'm' of the title at (row=0, col=0)"
    );
    // The '5' immediately follows.
    assert_eq!(frame.cells[0][1].glyph, b'5');
    assert_eq!(frame.cells[0][2].glyph, b'T');
}

#[test]
fn help_renders_close_hint() {
    let theme = coldwire();
    let frame = render(&help_state(), &theme);
    // Row 15 is reserved for the `esc close` hint.
    assert_eq!(
        frame.cells[15][0].glyph, b'e',
        "expected 'e' of 'esc close' at (row=15, col=0)"
    );
    assert_eq!(frame.cells[15][1].glyph, b's');
    assert_eq!(frame.cells[15][2].glyph, b'c');
    // The hint is rendered in the theme's dim color.
    assert_eq!(frame.cells[15][0].fg, theme.palette.dim.0);
}

#[test]
fn help_renders_rich_binding_table() {
    let theme = coldwire();
    let frame = render(&help_state(), &theme);
    // The help widget has a 2-column layout with 14 body rows (28 binding
    // slots). The SECTIONS table in widgets/help.rs holds 33 entries:
    // 25 start with `;`, the rest are chord-free names like `tab`,
    // `up`, `esc`. The 40x16 frame renders ~15-25 `;`-prefixed bindings
    // visibly (subject to 2-col overflow from the right column's key
    // chord clipping into the desc).
    //
    // We assert: (a) at least 15 `;` cells across the body rows, and
    // (b) at least 80 printable body cells (so we catch regressions
    // that silently drop bindings).
    let mut semi_count = 0usize;
    let mut body_cells = 0usize;
    for row in 1..ROWS - 1 {
        for col in 0..COLS {
            let g = frame.cells[row][col].glyph;
            if g == b';' {
                semi_count += 1;
            }
            if g.is_ascii_graphic() {
                body_cells += 1;
            }
        }
    }
    assert!(
        semi_count >= 15,
        "expected at least 15 ';' bindings visible, found {semi_count}"
    );
    assert!(
        body_cells >= 80,
        "expected at least 80 printable body cells, found {body_cells}"
    );
}

#[test]
fn help_renders_in_accent_color() {
    let theme = coldwire();
    let frame = render(&help_state(), &theme);
    // The title `m5Tui v0.1.0 -- HOTKEYS` is in the theme's accent.
    assert_eq!(
        frame.cells[0][0].fg, theme.palette.accent.0,
        "expected the title to be in the theme's accent"
    );
}
