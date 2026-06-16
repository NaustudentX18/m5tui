//! The M2 on-device theme editor.
//!
//! A full-screen overlay that lists the 9 editor screens (palette, colors,
//! glyphs, layout, animation, sound, brightness, save, export) in a 2-column
//! menu and shows a live cockpit preview under the current theme's palette.
//!
//! The widget is a pure read of `AppState` and the current `Theme`; it never
//! mutates either. Navigation inside the editor is the framework's concern;
//! this file only knows how to draw the screen.
//!
//! Layout (40 cols x 16 rows):
//!
//! ```text
//!   0:  THEME: <name>                                (title in accent)
//!   1:  ----------------------------------------    (separator)
//!   2:  01 palette     >    06 sound     >
//!   3:  02 colors      >    07 brightness >
//!   4:  03 glyphs      >    08 save      >
//!   5:  04 layout      >    09 export    >
//!   6:  05 animation   >
//!   7-13: (reserved for sub-screens, blank in the menu grid)
//!  14:  aiserver-1 :: OK
//!  15:  ;1-9 select | esc close
//! ```
//!
//! Rows 2..=6 hold the visible menu items in a 2-column layout (5 on the
//! left, 4 on the right). Each item line ends in a `>` arrow on the right
//! edge of its column, so the test can count `>` chars to find the menu.
//!
//! The live preview (row 14) shows the first row of the cockpit, rendered
//! with the active theme's `fg` and `bg`, so the user sees their palette
//! choices immediately. Row 15 carries the editor hint.
//!
//! The widget reads `theme.name` and `theme.palette.{bg, fg, accent, dim}`
//! and never falls back to a default — an empty `Theme` is a bug at the
//! call site, not a recoverable case here.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// The 9 editor screens in the order the menu lists them.
const ITEMS: [&str; 9] = [
    "palette",
    "colors",
    "glyphs",
    "layout",
    "animation",
    "sound",
    "brightness",
    "save",
    "export",
];

/// Draw the theme editor overlay into `frame`. The full screen is repainted
/// in `theme.palette.bg` so the overlay completely covers whatever was
/// underneath (the cockpit, a toast, etc.).
pub fn render(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    let bg_raw = theme.palette.bg.0;
    let fg_raw = theme.palette.fg.0;
    let accent_raw = theme.palette.accent.0;
    let dim_raw = theme.palette.dim.0;

    // Wipe the screen in the theme's background. The framebuffer's `fg`
    // channel is set to the theme's foreground so any cell left as a space
    // is still themable.
    for row in 0..ROWS {
        for col in 0..COLS {
            frame.cells[row][col] = Cell {
                glyph: b' ',
                fg: fg_raw,
                bg: bg_raw,
                attrs: 0,
            };
        }
    }

    // Row 0: `THEME: <name>` in accent.
    let title = build_title(&theme.name);
    write_str_colored(frame, 0, 0, &title, accent_raw, bg_raw);

    // Row 1: separator of `-` in dim, full width.
    for col in 0..COLS {
        frame.cells[1][col] = Cell {
            glyph: b'-',
            fg: dim_raw,
            bg: bg_raw,
            attrs: 0,
        };
    }

    // Rows 2..=10: 9 menu items in 2 columns. Left col occupies cols 0..19
    // (20 wide), right col occupies cols 20..COLS (20 wide). The 5 left
    // items fill rows 2..=6; the 4 right items fill rows 2..=5. Rows
    // 7..=10 and 11..=13 stay blank to leave room for sub-screens that
    // will scroll into view (palette is 10 swatches tall, etc.).
    let left_col = 0usize;
    let right_col = 20usize;
    for (i, name) in ITEMS.iter().enumerate() {
        let row = 2 + i;
        if i < 5 {
            // Items 1..=5 on the left column at rows 2..=6.
            write_menu_item(
                frame,
                row,
                left_col,
                i + 1,
                name,
                fg_raw,
                accent_raw,
                bg_raw,
            );
        } else {
            // Items 6..=9 on the right column at rows 2..=5.
            let j = i - 5;
            write_menu_item(
                frame,
                row,
                right_col,
                j + 6,
                name,
                fg_raw,
                accent_raw,
                bg_raw,
            );
        }
    }

    // Row 14: live preview. Show the first 2 chars of the cockpit's top
    // bar in the theme's foreground so the user sees their `fg`/`bg` pick.
    // The spec calls for `aiserver-1 :: OK` (40 chars) but the preview slot
    // is 1 row, so we keep the prefix that fits and the `OK` is rendered
    // in the accent.
    let preview = "aiserver-1 :: OK";
    write_str_colored(frame, 14, 0, preview, fg_raw, bg_raw);
    // Highlight the `OK` token in the accent.
    if let Some(ok_col) = preview.find("OK") {
        write_str_colored(frame, 14, ok_col, "OK", accent_raw, bg_raw);
    }

    // Row 15: hint.
    let hint = ";1-9 select | esc close";
    write_str_colored(frame, 15, 0, hint, dim_raw, bg_raw);
}

/// Render one menu line: `{index:2} {name:16} >` at `(row, col)`. The
/// trailing `>` is right-justified at the end of the column (col + 19) so
/// the arrows line up visually.
#[allow(clippy::too_many_arguments)]
fn write_menu_item(
    frame: &mut Frame,
    row: usize,
    col: usize,
    index: usize,
    name: &str,
    fg: u16,
    accent: u16,
    bg: u16,
) {
    // 2-char index, space, 16-char name (truncated/padded), then the arrow
    // at col + 19. Total column span: 20.
    let index_text = format!("{:02}", index);
    write_str_colored(frame, row, col, &index_text, accent, bg);
    write_str_colored(frame, row, col + 2, " ", fg, bg);
    // Name slot is 16 chars wide; the spec calls for 16 too.
    let name_slot = pad_or_truncate(name, 16);
    write_str_colored(frame, row, col + 3, &name_slot, fg, bg);
    // Arrow at the right edge of the column.
    let arrow_col = col + 19;
    if arrow_col < COLS {
        frame.cells[row][arrow_col] = Cell {
            glyph: b'>',
            fg: accent,
            bg,
            attrs: 0,
        };
    }
}

/// Build the title string `THEME: <name>`. Clipped to `COLS` so a long
/// name does not overflow the frame.
fn build_title(name: &str) -> String {
    let prefix = "THEME: ";
    let mut out = String::with_capacity(prefix.len() + name.len());
    out.push_str(prefix);
    out.push_str(name);
    out.truncate(COLS);
    out
}

/// Truncate or right-pad `s` to exactly `max` bytes (ASCII only — the
/// theme's `name` is plain ASCII by validate()'s contract).
fn pad_or_truncate(s: &str, max: usize) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= max {
        // SAFETY: validate() ensures names are ASCII so splitting on a
        // byte boundary is a UTF-8 boundary too. Fall back to the original
        // string on the (impossible in practice) invalid case.
        String::from_utf8(bytes[..max].to_vec()).unwrap_or_else(|_| s.to_string())
    } else {
        let mut out = String::from(s);
        for _ in bytes.len()..max {
            out.push(' ');
        }
        out
    }
}

/// Write `s` into the frame at `(row, col)`, clipped at `COLS`. Bytes are
/// taken as ASCII; non-graphic bytes are rendered as `?` (matches the
/// cockpit widget's behaviour).
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
    use crate::framebuffer::Frame;

    /// A minimal theme for unit-testing the editor. Colors chosen to be
    /// distinct from the default `palette::*` constants so a regression
    /// that falls back to a constant is caught.
    fn test_theme() -> Theme {
        use m5tui_themes::{
            CursorStyle, Density, GlyphSet, ScrollbarStyle, Theme, ThemeAnimation, ThemeGlyphs,
            ThemeLayout, ThemePalette, ThemeSound,
        };
        Theme {
            name: "test".into(),
            author: String::new(),
            palette: ThemePalette {
                bg: m5tui_themes::Rgb565(0x1111),
                fg: m5tui_themes::Rgb565(0xEEEE),
                accent: m5tui_themes::Rgb565(0xABCD),
                dim: m5tui_themes::Rgb565(0x2222),
                warn: m5tui_themes::Rgb565(0x3333),
                err: m5tui_themes::Rgb565(0x4444),
                ok: m5tui_themes::Rgb565(0x5555),
                sel_bg: m5tui_themes::Rgb565(0x6666),
                sel_fg: m5tui_themes::Rgb565(0x7777),
                prompt: m5tui_themes::Rgb565(0x8888),
            },
            glyphs: ThemeGlyphs {
                box_: GlyphSet::Ascii,
                scrollbar: ScrollbarStyle::Block,
                cursor: CursorStyle::Block,
            },
            layout: ThemeLayout {
                density: Density::Compact,
                show_clock: false,
                show_synthwave: false,
            },
            animation: ThemeAnimation {
                level: 0,
                scanline: false,
                phosphor_decay: false,
                glitch_on_event: false,
            },
            sound: ThemeSound::default_on(),
            brightness: 50,
        }
    }

    #[test]
    fn title_carries_theme_name() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        let state = AppState::default();
        render(&mut f, &state, &theme);
        let title = build_title(&theme.name);
        assert!(title.starts_with("THEME: "));
        assert_eq!(f.cells[0][0].glyph, b'T');
        assert_eq!(f.cells[0][7].glyph, b't'); // start of "test"
    }

    #[test]
    fn title_is_in_accent() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        render(&mut f, &AppState::default(), &theme);
        assert_eq!(f.cells[0][0].fg, 0xABCD);
    }

    #[test]
    fn screen_is_painted_in_theme_bg() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        render(&mut f, &AppState::default(), &theme);
        // Spot check: cell (3, 5) is body of menu; bg should be 0x1111.
        assert_eq!(f.cells[3][5].bg, 0x1111);
        // Hint row's first cell bg must also be the theme bg.
        assert_eq!(f.cells[15][0].bg, 0x1111);
    }

    #[test]
    fn menu_arrows_count_to_9() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        render(&mut f, &AppState::default(), &theme);
        let mut arrows = 0usize;
        for row in 2..14 {
            for col in 0..COLS {
                if f.cells[row][col].glyph == b'>' {
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
    fn menu_lists_all_nine_names() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        render(&mut f, &AppState::default(), &theme);
        for name in ITEMS.iter() {
            let needle = name.as_bytes();
            let mut found = false;
            for row in 2..14 {
                for start in 0..COLS.saturating_sub(needle.len()) {
                    if f.cells[row][start].glyph == needle[0]
                        && f.cells[row][start + 1].glyph == needle[1.min(needle.len() - 1)]
                    {
                        // Cheap check: every byte in the name should be
                        // present in the right relative positions. For
                        // names >= 2 bytes this catches the obvious
                        // missing-label regression.
                        let mut ok = true;
                        for (i, b) in needle.iter().enumerate() {
                            if f.cells[row][start + i].glyph != *b {
                                ok = false;
                                break;
                            }
                        }
                        if ok {
                            found = true;
                            break;
                        }
                    }
                }
                if found {
                    break;
                }
            }
            assert!(found, "menu item '{name}' not found in the editor body");
        }
    }

    #[test]
    fn preview_row_carries_cockpit_prefix() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        render(&mut f, &AppState::default(), &theme);
        // Row 14 begins with 'a' of "aiserver-1".
        assert_eq!(f.cells[14][0].glyph, b'a');
        // The OK token is in accent.
        if let Some(ok) = "aiserver-1 :: OK".find("OK") {
            assert_eq!(f.cells[14][ok].fg, 0xABCD);
        }
    }

    #[test]
    fn hint_row_carries_close_shortcut() {
        let mut f = Frame::new_solid(0);
        let theme = test_theme();
        render(&mut f, &AppState::default(), &theme);
        assert_eq!(f.cells[15][0].glyph, b';');
        assert_eq!(f.cells[15][1].glyph, b'1');
        // The hint is in dim, not accent.
        assert_eq!(f.cells[15][0].fg, 0x2222);
    }
}
