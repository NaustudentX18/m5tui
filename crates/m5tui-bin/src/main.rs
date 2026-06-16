//! m5Tui binary shim. M1: drive the three M1 widgets (cockpit, palette,
//! help) in sequence and print a one-line summary for each. All logic
//! lives in `m5tui-core`.

use m5tui_core::*;

const RGBA_LEN: usize = 240 * 135 * 4;

fn render_and_check(state: &AppState) {
    let frame = render(state);
    let buf = sim::render_to_rgba(&frame);
    assert_eq!(
        buf.len(),
        RGBA_LEN,
        "expected {RGBA_LEN} bytes for 240x135 RGBA"
    );
}

fn main() {
    // 1. Cockpit.
    let s1 = AppState::default();
    render_and_check(&s1);
    println!("m5Tui v0.1.0 -- Cockpit");

    // 2. Palette.
    let s2 = step(s1, Event::Key(KeyAction::Palette)).0;
    render_and_check(&s2);
    println!("m5Tui v0.1.0 -- Palette");

    // 3. Help.
    let s3 = step(s2, Event::Key(KeyAction::Help)).0;
    render_and_check(&s3);
    println!("m5Tui v0.1.0 -- Help");
}
