//! OMP cards widget — right pane of the cockpit while a session is live.
//!
//! Layout — the widget renders into the 40-col × 16-row frame but only
//! touches the right pane (`col = RIGHT_PANE_START..COLS`) on rows
//! 0..=12 plus row 14. The cockpit's left pane (agent list) and its
//! hint-bar / separator / prompt rows remain visible underneath.
//!
//! ```text
//!   row  0 cols 20..39: think: <state.omp_thinking text>
//!                        (blank if None)
//!   rows 1-4 cols 20..39: todos  -- "[ ] foo" / "[x] foo"
//!                          (up to 4, oldest-first)
//!   rows 5-6 cols 20..39: subagents -- "> task1" / "> task2"
//!                          (up to 2, oldest-first)
//!   rows 7-12 cols 20..39: cards -- newest first
//!                          "▶ id  tool(args)" for running
//!                          "✓ id  tool (0.4s)" for ended
//!   row 14 cols 20..39: most recent answer (truncated)
//!   other cells: untouched (cockpit left pane, hint, separator, prompt)
//! ```
//!
//! `▶` and `✓` share the first UTF-8 byte (`0xE2`); the 5x8 atlas
//! renders that byte as a blank cell. Tests can still detect the marker
//! by reading `cell.glyph`. The visual difference between running and
//! ended cards comes from the fg color (accent vs ok).
//!
//! The widget is pure: it reads `AppState` and mutates `Frame`; no
//! allocation, no I/O on the hot path beyond the `format!` strings
//! used to assemble the per-card line.

use crate::app::{AppState, OmpCard};
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

/// First column of the right pane in the cockpit (matches the
/// "SESSION" header at row 1 in `super::cockpit`).
const RIGHT_PANE_START: usize = 20;

/// Row used for the `think:` line.
const THINK_ROW: usize = 0;
/// First todo row.
const TODO_START: usize = 1;
/// Maximum number of todo rows rendered.
const TODO_ROWS: usize = 4;
/// First subagent row.
const SUB_START: usize = 5;
/// Maximum number of subagent rows rendered.
const SUB_ROWS: usize = 2;
/// First card row.
const CARD_START: usize = 7;
/// Maximum number of card rows rendered.
const CARD_ROWS: usize = 6;
/// Row used for the most-recent answer line.
const ANSWER_ROW: usize = 14;

/// Marker byte used for both running and ended cards. Both `▶` (U+25B6)
/// and `✓` (U+2713) share the first UTF-8 byte `0xE2`. The fg color
/// (`accent` vs `ok`) is what makes the running vs ended state
/// visually distinct on-device.
const CARD_MARKER: u8 = 0xE2;

/// Render the OMP cards panel into the right pane of `frame`.
/// Columns `RIGHT_PANE_START..COLS` on rows 0..=12 plus row 14 are
/// overwritten; everything else (left pane, hint, separator, prompt)
/// is left untouched.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    let dim = theme.palette.dim.0;
    let ok = theme.palette.ok.0;

    // Row 0: think: <text>
    if let Some(text) = state.omp_thinking.as_deref() {
        let line = format!("think: {text}");
        write_line_colored(frame, THINK_ROW, &line, accent, bg);
    }

    // Rows 1-4: todos, oldest-first, capped at TODO_ROWS.
    for (i, todo) in state.omp_todos.iter().take(TODO_ROWS).enumerate() {
        let row = TODO_START + i;
        let marker = if todo.done { b'x' } else { b' ' };
        let line = format!("[{}] {}", marker as char, todo.text);
        write_line_colored(frame, row, &line, fg, bg);
    }

    // Rows 5-6: subagents, oldest-first, capped at SUB_ROWS.
    for (i, sub) in state.omp_subagents.iter().take(SUB_ROWS).enumerate() {
        let row = SUB_START + i;
        let line = format!("> {}", sub.task);
        write_line_colored(frame, row, &line, dim, bg);
    }

    // Rows 7-12: cards, newest-first, capped at CARD_ROWS.
    // The reducer stores cards oldest-first, so reverse for display.
    for (i, card) in state.omp_cards.iter().rev().take(CARD_ROWS).enumerate() {
        let row = CARD_START + i;
        let color = if card.ended.is_some() { ok } else { accent };
        let tail = card_tail(card);
        // Two cells for the marker glyph and a trailing space, then the
        // rest of the line.
        if RIGHT_PANE_START < COLS {
            frame.cells[row][RIGHT_PANE_START] = Cell {
                glyph: CARD_MARKER,
                fg: color,
                bg,
                attrs: 0,
            };
            frame.cells[row][RIGHT_PANE_START + 1] = Cell {
                glyph: b' ',
                fg: color,
                bg,
                attrs: 0,
            };
            write_str_colored(frame, row, RIGHT_PANE_START + 2, &tail, fg, bg);
        }
    }

    // Row 14: most recent answer (last entry in the answers deque).
    if let Some(answer) = state.omp_answers.back() {
        write_line_colored(frame, ANSWER_ROW, answer, fg, bg);
    }
}

/// Format the tail of a card line (`id  tool(args)` for running,
/// `id  tool (0.4s)` for ended).
fn card_tail(card: &OmpCard) -> String {
    let tool = if card.args.is_empty() {
        card.tool.clone()
    } else {
        let parts: Vec<String> = card.args.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("{}({})", card.tool, parts.join(","))
    };
    if let (Some(start), Some(end)) = (card.started, card.ended) {
        let secs = end.saturating_sub(start) as f32 / 1000.0;
        format!("{}  {} ({:.1}s)", card.id, tool, secs)
    } else {
        format!("{}  {}", card.id, tool)
    }
}

/// Write `s` into the right pane of `row`, clipped at `COLS`.
fn write_line_colored(frame: &mut Frame, row: usize, s: &str, fg: u16, bg: u16) {
    write_str_colored(frame, row, RIGHT_PANE_START, s, fg, bg);
}

/// Write `s` into the frame at `(row, col)` byte-by-byte, clipped at
/// `COLS`. Non-printable bytes are rendered as `?`.
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
    use crate::app::{OmpSubagent, OmpTodo};

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    #[test]
    fn running_card_uses_running_marker() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let mut state = AppState::default();
        state.omp_cards.push(OmpCard {
            id: "a1".into(),
            tool: "read_file".into(),
            args: vec![("path".into(), "/etc/hosts".into())],
            started: Some(100),
            ended: None,
            output: None,
        });
        render(&mut f, &state, &theme);
        // The marker cell on row 7, col 20 should be the `▶` byte (0xE2).
        assert_eq!(f.cells[7][RIGHT_PANE_START].glyph, 0xE2);
    }

    #[test]
    fn thinking_text_renders_on_row_zero() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let state = AppState {
            omp_thinking: Some("reasoning about the user request".into()),
            ..Default::default()
        };
        render(&mut f, &state, &theme);
        // Row 0 right pane: `think: <text>`.
        // t h i n k : _ r e a s o n i n g ...
        // 0 1 2 3 4 5 6 7 8 ...
        assert_eq!(f.cells[0][RIGHT_PANE_START].glyph, b't');
        assert_eq!(f.cells[0][RIGHT_PANE_START + 4].glyph, b'k');
        assert_eq!(f.cells[0][RIGHT_PANE_START + 5].glyph, b':');
        assert_eq!(f.cells[0][RIGHT_PANE_START + 6].glyph, b' ');
        assert_eq!(f.cells[0][RIGHT_PANE_START + 7].glyph, b'r');
    }

    #[test]
    fn todo_done_marker_is_x() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let mut state = AppState::default();
        state.omp_todos.push(OmpTodo {
            id: "t1".into(),
            text: "ship".into(),
            done: true,
        });
        render(&mut f, &state, &theme);
        // Row 1 right pane: `[x] ship` -> col 20 '[', col 21 'x', col 22 ']'.
        assert_eq!(f.cells[1][RIGHT_PANE_START].glyph, b'[');
        assert_eq!(f.cells[1][RIGHT_PANE_START + 1].glyph, b'x');
        assert_eq!(f.cells[1][RIGHT_PANE_START + 2].glyph, b']');
    }

    #[test]
    fn subagent_renders_with_leading_gt() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let mut state = AppState::default();
        state.omp_subagents.push(OmpSubagent {
            id: "s1".into(),
            task: "scan".into(),
        });
        render(&mut f, &state, &theme);
        // Row 5 right pane starts with '>' (subagent prefix).
        assert_eq!(f.cells[5][RIGHT_PANE_START].glyph, b'>');
        assert_eq!(f.cells[5][RIGHT_PANE_START + 1].glyph, b' ');
        assert_eq!(f.cells[5][RIGHT_PANE_START + 2].glyph, b's');
    }
}
