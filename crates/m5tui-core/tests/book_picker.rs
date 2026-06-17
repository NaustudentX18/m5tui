//! Render tests for the book picker widget.

use m5tui_core::app::{AppState, SpellSummary};
use m5tui_core::framebuffer::Frame;
use m5tui_core::widgets::book_picker;

fn coldwire() -> m5tui_themes::Theme {
    m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
}

fn two_spells() -> Vec<SpellSummary> {
    vec![
        SpellSummary {
            id: "s1".into(),
            key: "ask".into(),
            help: "Ask the agent a question".into(),
        },
        SpellSummary {
            id: "s2".into(),
            key: "plan".into(),
            help: "Draft a plan for the task".into(),
        },
    ]
}

/// Returns true if every byte of `needle` appears consecutively
/// somewhere on the given row.
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
fn title_and_both_spell_keys_visible() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = m5tui_core::AppState {
        book_spells: two_spells(),
        ..Default::default()
    };
    book_picker::render(&mut frame, &state, &theme);

    // Title `Book` on row 0.
    assert!(row_contains(&frame, 0, "Book"), "expected `Book` on row 0",);
    // First key `ask` on row 1.
    assert!(row_contains(&frame, 1, "ask"), "expected `ask` on row 1",);
    // Second key `plan` on row 2.
    assert!(row_contains(&frame, 2, "plan"), "expected `plan` on row 2",);
}

#[test]
fn empty_spells_shows_no_spells_message() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = AppState::default();
    book_picker::render(&mut frame, &state, &theme);
    assert!(
        row_contains(&frame, 1, "No spells"),
        "expected `No spells` on row 1",
    );
}

#[test]
fn highlighted_index_shows_help_on_its_own_row() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = m5tui_core::AppState {
        book_spells: two_spells(),
        picker_index: 1,
        ..Default::default()
    };
    book_picker::render(&mut frame, &state, &theme);
    // The second spell's help text starts on row 2.
    assert!(
        row_contains(&frame, 2, "Draft a plan"),
        "expected `Draft a plan` on row 2",
    );
}
