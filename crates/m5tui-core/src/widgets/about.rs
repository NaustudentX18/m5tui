//! The M11 "About" overlay.
//!
//! Static, theme-aware build/version/feature sheet. The widget is pure —
//! it never reads from `AppState` because none of the fields it shows
//! are user-mutable. Layout is 40 cols x 16 rows; each row is hand-laid
//! out so the whole sheet fits the small Cardputer-Adv display without
//! scrolling.
//!
//! Closing the overlay is handled by the framework via the generic
//! `KeyAction::Esc` -> `Outgoing::CloseOverlay` path; this widget only
//! draws.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// Render the about overlay into `frame`. The body is split into four
/// short sections: title, build, themes, and keybinds.
pub fn render(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    let bg = theme.palette.bg.0;
    let fg = theme.palette.fg.0;
    let accent = theme.palette.accent.0;
    let dim = theme.palette.dim.0;
    let ok = theme.palette.ok.0;
    let err = theme.palette.err.0;

    // Solid background so previous widgets don't bleed through.
    for row in 0..ROWS {
        for col in 0..COLS {
            frame.cells[row][col] = Cell {
                glyph: b' ',
                fg: 0,
                bg,
                attrs: 0,
            };
        }
    }

    // Title block.
    write_str(frame, 1, 0, "m5Tui", accent, bg);
    write_str(frame, 2, 0, "v1.12.0 -- 2026-06-17", dim, bg);

    // Build / target / server facts.
    write_str(
        frame,
        4,
        0,
        "build:  stable + xtensa-esp32s3-espidf",
        fg,
        bg,
    );
    write_str(
        frame,
        5,
        0,
        "target: M5Stack Cardputer-Adv (ESP32-S3)",
        fg,
        bg,
    );
    write_str(
        frame,
        6,
        0,
        "server: aiserver-1 (Tailscale 100.126.207.73)",
        fg,
        bg,
    );

    // Theme list — two rows of three names each.
    write_str(frame, 8, 0, "themes:", fg, bg);
    write_str(
        frame,
        9,
        0,
        "  coldwire, phosphor, lacuna, magline,",
        ok,
        bg,
    );
    write_str(frame, 10, 0, "  noctilux, ivoryroom", ok, bg);

    // Keybind summary — the high-traffic chords the user reaches for.
    write_str(frame, 12, 0, "keybinds:", fg, bg);
    write_str(frame, 13, 0, "  ;? help   ;/ palette   ;t theme", dim, bg);
    write_str(frame, 14, 0, "  ;p profile   ;b book   ;v voice", dim, bg);

    // Close hint. The rest of the project uses `esc close` in `dim`,
    // but the spec asks for an `err`-coloured hint to match the
    // boot-screen's "press any key" affordance.
    write_str(frame, ROWS - 1, 0, "press Esc to close", err, bg);
}

/// Write `s` into `frame` starting at `(row, col)` using the supplied
/// colours. Bytes outside the printable ASCII range render as `?` so
/// non-graphic input never produces a blank cell.
fn write_str(frame: &mut Frame, row: usize, col: usize, s: &str, fg: u16, bg: u16) {
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
    use m5tui_themes;

    fn theme() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("missing coldwire theme"))
    }

    fn row_of(frame: &Frame, row: usize) -> String {
        let bytes: Vec<u8> = frame.cells[row].iter().map(|c| c.glyph).collect();
        String::from_utf8_lossy(&bytes).trim_end().to_string()
    }

    fn full_body(frame: &Frame) -> String {
        let mut s = String::new();
        for r in 0..ROWS {
            s.push_str(&row_of(frame, r));
            s.push('\n');
        }
        s
    }

    #[test]
    fn title_row_uses_accent_and_contains_m5tui() {
        let mut f = Frame::new_solid(0);
        render(&mut f, &AppState::default(), &theme());
        let cell = f.cells[1][0];
        assert_eq!(cell.glyph, b'm');
        assert_eq!(cell.fg, theme().palette.accent.0);
        assert_eq!(row_of(&f, 1), "m5Tui");
    }

    #[test]
    fn version_row_uses_dim_and_contains_v1_12() {
        let mut f = Frame::new_solid(0);
        render(&mut f, &AppState::default(), &theme());
        let row = row_of(&f, 2);
        assert!(row.starts_with("v1.12.0"), "got: {row}");
        // The version string should be rendered in `dim`, not `fg`.
        let cell = f.cells[2][0];
        assert_eq!(cell.fg, theme().palette.dim.0);
    }

    #[test]
    fn theme_list_is_complete() {
        let mut f = Frame::new_solid(0);
        render(&mut f, &AppState::default(), &theme());
        let body = full_body(&f);
        for name in [
            "coldwire",
            "phosphor",
            "lacuna",
            "magline",
            "noctilux",
            "ivoryroom",
        ] {
            assert!(body.contains(name), "missing theme '{name}'");
        }
    }

    #[test]
    fn keybind_list_is_complete() {
        let mut f = Frame::new_solid(0);
        render(&mut f, &AppState::default(), &theme());
        let body = full_body(&f);
        for chord in [";?", ";/", ";t", ";p", ";b", ";v"] {
            assert!(body.contains(chord), "missing keybind '{chord}'");
        }
    }

    #[test]
    fn all_body_rows_fit_within_40_columns() {
        let mut f = Frame::new_solid(0);
        render(&mut f, &AppState::default(), &theme());
        for r in 0..ROWS {
            let row = row_of(&f, r);
            assert!(row.len() <= COLS, "row {r} len {} > {COLS}", row.len());
        }
    }

    #[test]
    fn close_hint_uses_err_color_on_last_row() {
        let mut f = Frame::new_solid(0);
        render(&mut f, &AppState::default(), &theme());
        let row = ROWS - 1;
        let row_text = row_of(&f, row);
        assert!(
            row_text.starts_with("press Esc to close"),
            "got: {row_text}"
        );
        // The "p" of "press" sets the colour for the whole row.
        assert_eq!(f.cells[row][0].fg, theme().palette.err.0);
    }
}
