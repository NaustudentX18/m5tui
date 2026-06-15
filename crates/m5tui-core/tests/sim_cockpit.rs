//! Sim cockpit tests: verify the sim backend produces the right shape, is
//! deterministic, and renders the M0 title at the expected location.

use m5tui_core::*;

#[test]
fn sim_cockpit_is_240x135_rgba() {
    let state = AppState::default();
    let frame = render(&state);
    let buf = render_to_rgba(&frame);
    assert_eq!(
        buf.len(),
        240 * 135 * 4,
        "expected 129600 bytes for 240x135 RGBA"
    );
    assert!(!buf.is_empty());
    // The title is drawn in cyan, so at least one pixel must be non-zero.
    assert!(
        buf.chunks_exact(4).any(|p| p[3] == 0xFF),
        "expected at least one opaque pixel from the title"
    );
}

#[test]
fn sim_cockpit_is_deterministic() {
    let state = AppState::default();
    let frame = render(&state);
    let buf1 = render_to_rgba(&frame);
    let buf2 = render_to_rgba(&frame);
    assert_eq!(buf1, buf2, "render_to_rgba must be deterministic");
}

#[test]
fn sim_cockpit_non_zero_alpha_at_centre() {
    // Title "m5Tui v0.1.0" is 12 chars: start_col = (40 - 12) / 2 = 14,
    // start_row = 16 / 2 = 8. The 'm' at (row=8, col=14) is the first
    // char; its pixel (col=2, row=4) within the cell maps to global
    // pixel (x=86, y=68).
    let state = AppState::default();
    let frame = render(&state);
    let buf = render_to_rgba(&frame);
    let pixel_index = (68 * 240 + 86) * 4;
    let alpha = buf[pixel_index + 3];
    assert_eq!(
        alpha, 0xFF,
        "expected the title pixel to be opaque, got alpha={alpha}"
    );
}

#[test]
fn reduce_tick_increments_frame_count() {
    let s = AppState::default();
    let s2 = reduce(s, Event::Tick);
    assert_eq!(s2.frame_count, 1);
    let s3 = reduce(s2, Event::Tick);
    assert_eq!(s3.frame_count, 2);
}

#[test]
fn reduce_quit_sets_should_quit() {
    let s = AppState::default();
    let s2 = reduce(s, Event::Quit);
    assert!(s2.should_quit);
}
