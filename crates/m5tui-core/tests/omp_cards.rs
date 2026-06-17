//! Render tests for the OMP cards widget (right pane of the cockpit).
//!
//! Two cards are pushed into `state.omp_cards` — one running, one ended —
//! and the rendered 40x16 frame is scanned for the `▶` glyph byte.
//! A second test sets `state.omp_thinking` and checks the `think:` line.

use m5tui_core::app::{AppState, OmpCard, OmpTodo};
use m5tui_core::framebuffer::Frame;
use m5tui_core::widgets::omp_cards;

fn coldwire() -> m5tui_themes::Theme {
    m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
}

/// Scan the frame for a glyph equal to `target`. Returns the first
/// `(row, col)` where it appears, or `None`.
fn find_glyph(frame: &Frame, target: u8) -> Option<(usize, usize)> {
    for row in 0..frame.cells.len() {
        for col in 0..frame.cells[row].len() {
            if frame.cells[row][col].glyph == target {
                return Some((row, col));
            }
        }
    }
    None
}

/// Scan a single row for a substring, returning `true` if every byte
/// of `needle` appears consecutively starting at some column.
fn row_contains(frame: &Frame, row: usize, needle: &str) -> bool {
    let bytes = needle.as_bytes();
    let cols = frame.cells[row].len();
    if bytes.is_empty() || bytes.len() > cols {
        return false;
    }
    for start in 0..=(cols - bytes.len()) {
        let mut ok = true;
        for (i, b) in bytes.iter().enumerate() {
            if frame.cells[row][start + i].glyph != *b {
                ok = false;
                break;
            }
        }
        if ok {
            return true;
        }
    }
    false
}

#[test]
fn two_cards_render_running_marker() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let mut state = AppState::default();
    state.omp_cards.push(OmpCard {
        id: "a".into(),
        tool: "shell".into(),
        args: vec![("cmd".into(), "ls".into())],
        started: Some(100),
        ended: None,
        output: None,
    });
    state.omp_cards.push(OmpCard {
        id: "b".into(),
        tool: "read_file".into(),
        args: vec![("path".into(), "/etc/hosts".into())],
        started: Some(120),
        ended: Some(180),
        output: Some("ok".into()),
    });
    omp_cards::render(&mut frame, &state, &theme);
    // The first UTF-8 byte of `▶` is 0xE2.
    assert!(
        find_glyph(&frame, 0xE2).is_some(),
        "expected a `▶` glyph in the rendered frame",
    );
}

#[test]
fn thinking_text_renders_think_label() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = m5tui_core::AppState {
        omp_thinking: Some("working out the plan".into()),
        ..Default::default()
    };
    omp_cards::render(&mut frame, &state, &theme);
    assert!(
        row_contains(&frame, 0, "think:"),
        "expected `think:` on row 0",
    );
}

#[test]
fn todos_render_with_checkbox_marker() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let mut state = AppState::default();
    state.omp_todos.push(OmpTodo {
        id: "1".into(),
        text: "draft".into(),
        done: false,
    });
    state.omp_todos.push(OmpTodo {
        id: "2".into(),
        text: "ship".into(),
        done: true,
    });
    omp_cards::render(&mut frame, &state, &theme);
    assert!(row_contains(&frame, 1, "[ ]"), "expected `[ ]` on row 1",);
    assert!(row_contains(&frame, 2, "[x]"), "expected `[x]` on row 2",);
}
