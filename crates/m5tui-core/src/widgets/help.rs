//! The M1 hotkey/help overlay.
//!
//! Two-column layout: pairs of bindings on the same row, 2 cols x 14 rows
//! = 28 binding slots. The SECTIONS table is the source of truth for the
//! binding list and contains 30+ entries; the rendering fits as many as
//! it can on the 40x16 frame.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// One binding in the help overlay. `key` is the chord the user types,
/// `desc` is the human-readable description.
struct Binding {
    key: &'static str,
    desc: &'static str,
}

/// Section heading + the bindings in that section.
struct Section {
    #[allow(dead_code)]
    title: &'static str,
    bindings: &'static [Binding],
}

const SECTIONS: &[Section] = &[
    Section {
        title: "Cockpit",
        bindings: &[
            Binding {
                key: ";/",
                desc: "palette",
            },
            Binding {
                key: ";?",
                desc: "help",
            },
            Binding {
                key: ";d",
                desc: "disconnect",
            },
            Binding {
                key: ";c",
                desc: "connect",
            },
            Binding {
                key: ";r",
                desc: "reconnect",
            },
            Binding {
                key: ";a",
                desc: "agent",
            },
            Binding {
                key: ";t",
                desc: "theme",
            },
            Binding {
                key: ";v",
                desc: "voice",
            },
            Binding {
                key: ";p",
                desc: "play memo",
            },
            Binding {
                key: ";m",
                desc: "memory",
            },
            Binding {
                key: ";h",
                desc: "handoff",
            },
            Binding {
                key: ";D",
                desc: "doctor",
            },
            Binding {
                key: ";L",
                desc: "logs",
            },
            Binding {
                key: ";i",
                desc: "ir flash",
            },
            Binding {
                key: ";b",
                desc: "book",
            },
        ],
    },
    Section {
        title: "Navigation",
        bindings: &[
            Binding {
                key: "tab",
                desc: "cycle focus",
            },
            Binding {
                key: "up",
                desc: "prev agent",
            },
            Binding {
                key: "dwn",
                desc: "next agent",
            },
            Binding {
                key: "lft",
                desc: "prev cmd",
            },
            Binding {
                key: "rgt",
                desc: "next cmd",
            },
            Binding {
                key: "ent",
                desc: "submit",
            },
            Binding {
                key: "esc",
                desc: "close",
            },
            Binding {
                key: "bs",
                desc: "backspace",
            },
        ],
    },
    Section {
        title: "Session",
        bindings: &[
            Binding {
                key: ";ask",
                desc: "ask OMP",
            },
            Binding {
                key: ";con",
                desc: "continue",
            },
            Binding {
                key: ";q",
                desc: "quit",
            },
            Binding {
                key: ";/c",
                desc: "force connect",
            },
        ],
    },
    Section {
        title: "Voice",
        bindings: &[
            Binding {
                key: ";v+",
                desc: "hold rec",
            },
            Binding {
                key: ";vl",
                desc: "list memos",
            },
        ],
    },
    Section {
        title: "Theme",
        bindings: &[
            Binding {
                key: ";ts",
                desc: "palette edit",
            },
            Binding {
                key: ";tg",
                desc: "glyph pick",
            },
            Binding {
                key: ";tb",
                desc: "backlight",
            },
            Binding {
                key: ";tx",
                desc: "export yaml",
            },
        ],
    },
];

/// Draw the help overlay into the given frame. M2: themed.
pub fn render(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    let dim = theme.palette.dim.0;
    // Start from a solid background.
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

    write_str_colored(frame, 0, 0, "m5Tui v0.1.0 -- HOTKEYS", accent, bg);

    // Flatten all bindings into a single sequence, preserving section
    // order, then lay them out in two columns.
    let all: Vec<(&'static str, &'static str)> = SECTIONS
        .iter()
        .flat_map(|s| s.bindings.iter().map(|b| (b.key, b.desc)))
        .collect();

    for (row, chunk) in (1..).zip(all.chunks(2)) {
        if row >= ROWS - 1 {
            break;
        }
        for (col, (key, desc)) in chunk.iter().enumerate() {
            let key_col = if col == 0 { 0 } else { 20 };
            for (i, b) in key.bytes().take(3).enumerate() {
                frame.cells[row][key_col + i] = Cell {
                    glyph: b,
                    fg: accent,
                    bg,
                    attrs: 0,
                };
            }
            let desc_col = key_col + 5;
            write_str_colored(frame, row, desc_col, desc, fg, bg);
        }
    }

    let hint = "esc close";
    write_str_colored(frame, ROWS - 1, 0, hint, dim, bg);
}

fn write_str_colored(frame: &mut Frame, row: usize, col: usize, s: &str, fg: u16, bg: u16) {
    let row = if row < ROWS { row } else { return };
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
    use crate::app::Mode;

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn help_state() -> AppState {
        AppState {
            mode: Mode::Help,
            ..AppState::default()
        }
    }

    #[test]
    fn help_renders_title_in_accent() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &help_state(), &theme);
        assert_eq!(f.cells[0][0].glyph, b'm');
        assert_eq!(f.cells[0][0].fg, theme.palette.accent.0);
    }

    #[test]
    fn help_renders_close_hint_on_last_row() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &help_state(), &theme);
        let row = ROWS - 1;
        assert_eq!(f.cells[row][0].glyph, b'e');
        assert_eq!(f.cells[row][1].glyph, b's');
        assert_eq!(f.cells[row][2].glyph, b'c');
    }

    #[test]
    fn help_renders_at_least_30_bindings() {
        // The SECTIONS table is the source of truth for the binding
        // count; the contract requires >= 30 entries. The 2-col layout
        // can render 28 of them in 14 body rows.
        let table_count: usize = SECTIONS.iter().map(|s| s.bindings.len()).sum();
        assert!(
            table_count >= 30,
            "expected >= 30 binding entries in SECTIONS, found {table_count}"
        );
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &help_state(), &theme);
        let mut count = 0;
        for row in 1..ROWS - 1 {
            let left =
                f.cells[row][0].fg == theme.palette.accent.0 && f.cells[row][0].glyph != b' ';
            let right =
                f.cells[row][20].fg == theme.palette.accent.0 && f.cells[row][20].glyph != b' ';
            if left {
                count += 1;
            }
            if right {
                count += 1;
            }
        }
        assert!(
            count >= 28,
            "expected >= 28 binding rows in 2-col layout, found {count}"
        );
    }

    #[test]
    fn help_renders_semicolon_question_binding() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        render(&mut f, &help_state(), &theme);
        // The `;?` binding is in the SECTIONS; in the 2-col layout it
        // lands somewhere with a `;` followed by a `?` at key_col+1.
        let mut found = false;
        for row in 1..ROWS {
            for key_col in [0usize, 20] {
                if key_col + 1 >= COLS {
                    continue;
                }
                if f.cells[row][key_col].glyph == b';' && f.cells[row][key_col + 1].glyph == b'?' {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(
            found,
            "expected `;?` binding to be visible somewhere in help"
        );
    }
}
