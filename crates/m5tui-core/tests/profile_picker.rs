//! Render tests for the profile picker widget.

use m5tui_core::app::{AppState, ProfileSummary};
use m5tui_core::framebuffer::Frame;
use m5tui_core::widgets::profile_picker;

fn coldwire() -> m5tui_themes::Theme {
    m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
}

fn two_profiles() -> Vec<ProfileSummary> {
    vec![
        ProfileSummary {
            id: "p1".into(),
            label: "default".into(),
            host: "aiserver-1".into(),
        },
        ProfileSummary {
            id: "p2".into(),
            label: "home".into(),
            host: "pc-ollama".into(),
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
fn title_and_both_profile_labels_visible() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = m5tui_core::AppState {
        profiles: two_profiles(),
        ..Default::default()
    };
    profile_picker::render(&mut frame, &state, &theme);

    // Title `Profiles` on row 0.
    assert!(
        row_contains(&frame, 0, "Profiles"),
        "expected `Profiles` on row 0",
    );
    // First label `default` on row 1.
    assert!(
        row_contains(&frame, 1, "default"),
        "expected `default` on row 1",
    );
    // Second label `home` on row 2.
    assert!(row_contains(&frame, 2, "home"), "expected `home` on row 2",);
    // First host `aiserver-1` on row 1.
    assert!(
        row_contains(&frame, 1, "aiserver-1"),
        "expected `aiserver-1` on row 1",
    );
}

#[test]
fn empty_profiles_shows_no_profiles_message() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = AppState::default();
    profile_picker::render(&mut frame, &state, &theme);
    assert!(
        row_contains(&frame, 1, "No profiles"),
        "expected `No profiles` on row 1",
    );
}

#[test]
fn highlighted_index_shows_host_on_its_own_row() {
    let theme = coldwire();
    let mut frame = Frame::new_solid(theme.palette.bg.0);
    let state = m5tui_core::AppState {
        profiles: two_profiles(),
        picker_index: 1,
        ..Default::default()
    };
    profile_picker::render(&mut frame, &state, &theme);
    // The second profile's host (`pc-ollama`) must appear on row 2
    // (its body row index, since highlight == 1).
    assert!(
        row_contains(&frame, 2, "pc-ollama"),
        "expected `pc-ollama` on row 2",
    );
}
