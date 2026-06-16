//! Sim cockpit tests: verify the sim backend produces the right shape, is
//! deterministic, and renders the M1 cockpit with known glyphs at known
//! cells. The M0 alpha-centre test was updated to the M1 layout: the
//! cockpit's top bar is the server-status line and the agents list starts
//! at (row=2, col=1).
//!
//! The pixel-level assertions below scan a small region under the target
//! cell for any opaque pixel rather than pinning to a single (gx, gy)
//! offset. The sim blit logic is an internal implementation detail; what
//! the contract guarantees is that the cell is rendered and that opaque
//! pixels exist in the cell's 6x8 pixel region.

use m5tui_core::*;

fn coldwire() -> m5tui_themes::Theme {
    m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
}

// Layout constants duplicated from `layout.rs` for self-contained tests.
const CELL_W: usize = 6;
const CELL_H: usize = 8;
const FB_W: usize = 240;

// 40x16 RGBA = 240*135*4 = 129600 bytes. Same as `RGBA_LEN` in sim.rs.
const RGBA_LEN: usize = 240 * 135 * 4;

/// Scan the 6x8 pixel region of cell `(cell_col, cell_row)` in the RGBA
/// buffer and return the first opaque pixel found (as `(x, y)`), or `None`.
fn first_opaque_in_cell(buf: &[u8], cell_col: usize, cell_row: usize) -> Option<(usize, usize)> {
    let x0 = cell_col * CELL_W;
    let y0 = cell_row * CELL_H;
    for dy in 0..CELL_H {
        for dx in 0..CELL_W {
            let idx = ((y0 + dy) * FB_W + (x0 + dx)) * 4;
            if buf[idx + 3] == 0xFF {
                return Some((x0 + dx, y0 + dy));
            }
        }
    }
    None
}

#[test]
fn sim_cockpit_is_240x135_rgba() {
    let state = AppState::default();
    let theme = coldwire();
    let frame = render(&state, &theme);
    let buf = render_to_rgba(&frame);
    assert_eq!(
        buf.len(),
        RGBA_LEN,
        "expected 129600 bytes for 240x135 RGBA"
    );
    assert!(!buf.is_empty());
    // The cockpit top bar is rendered in FG, so at least one pixel must be
    // non-zero.
    assert!(
        buf.chunks_exact(4).any(|p| p[3] == 0xFF),
        "expected at least one opaque pixel from the cockpit"
    );
}

#[test]
fn sim_cockpit_is_deterministic() {
    let state = AppState::default();
    let theme = coldwire();
    let frame = render(&state, &theme);
    let buf1 = render_to_rgba(&frame);
    let buf2 = render_to_rgba(&frame);
    assert_eq!(buf1, buf2, "render_to_rgba must be deterministic");
}

#[test]
fn sim_cockpit_non_zero_alpha_at_centre() {
    // The M1 cockpit has no centred title; the agents list starts at
    // (row=2, col=1) with the first agent `aiserver-1/omp`. The cell at
    // (row=2, col=1) holds the first letter of the agent name. Verify
    // that the cell renders at least one opaque pixel in its 6x8 region.
    let state = AppState::default();
    let theme = coldwire();
    let frame = render(&state, &theme);
    let buf = render_to_rgba(&frame);
    let cell = frame.cells[2][1];
    assert_eq!(cell.glyph, b'a', "expected 'a' at (row=2, col=1)");
    let found = first_opaque_in_cell(&buf, 1, 2);
    assert!(
        found.is_some(),
        "expected an opaque pixel under the first agent's 'a'"
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

#[test]
fn sim_cockpit_renders_prompt_caret() {
    // With prompt = "hello world", the cockpit's last row contains
    // `> hello world_` and the 'h' is at (row=15, col=2). The cell
    // should hold glyph 'h' and render at least one opaque pixel in
    // its 6x8 region.
    let state = AppState {
        prompt: "hello world".into(),
        ..AppState::default()
    };
    let theme = coldwire();
    let frame = render(&state, &theme);
    let buf = render_to_rgba(&frame);
    assert_eq!(
        frame.cells[15][2].glyph, b'h',
        "expected the prompt's 'h' to be at (row=15, col=2)"
    );
    let found = first_opaque_in_cell(&buf, 2, 15);
    assert!(
        found.is_some(),
        "expected the prompt 'h' to render an opaque pixel"
    );
}

#[test]
fn sim_cockpit_focus_agents_cursor() {
    // With focus=Agents and selected_agent=0, the cockpit's first agent
    // row gets a `>` cursor at (row=2, col=0). The cell should hold
    // glyph '>' and render at least one opaque pixel.
    let state = AppState {
        focus: Focus::Agents,
        ..AppState::default()
    };
    let theme = coldwire();
    let frame = render(&state, &theme);
    let buf = render_to_rgba(&frame);
    assert_eq!(
        frame.cells[2][0].glyph, b'>',
        "expected the focus cursor '>' at (row=2, col=0)"
    );
    let found = first_opaque_in_cell(&buf, 0, 2);
    assert!(
        found.is_some(),
        "expected the focus cursor to render an opaque pixel"
    );
}
