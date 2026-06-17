//! `m5tui-core` — the pure rendering core for the m5Tui firmware/sim.
//!
//! M0 scope: a tiny `AppState` with a pure reducer, a 40x16 framebuffer of
//! `Cell`s, a 5x8 ASCII glyph atlas, a `render` that centres the app title
//! on the screen, and a sim backend that converts a `Frame` to a 240x135
//! RGBA8888 byte buffer. No I/O, no allocations in the hot path beyond the
//! output buffer in `render_to_rgba`.
//!
//! M1 scope: the input layer (`keymap`, `KeyAction`, `ChordParser`),
//! the command palette (`palette::Command`, `BUILTINS`, `fuzzy_score`,
//! `filter`), extended `Event`/`Outgoing`/`Focus`/`Mode` enums, the
//! mock-app state, and `step` (the side-channel reducer).

pub mod app;
pub mod event;
pub mod framebuffer;
pub mod keymap;
pub mod layout;
pub mod palette;
pub mod render;
pub mod sim;
pub mod widgets;

/// The default theme for the M0/M1 sim (and as a sensible fallback
/// when no theme has been loaded yet). Coldwire is the spec's hero
/// theme; every other theme slots in the same way.
pub fn default_theme() -> m5tui_themes::Theme {
    m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
}

pub use app::{reduce, step, AppState, MockAgent, MockSession, Mode, Toast};
pub use event::{Event, Focus, KeyAction, Outgoing};
pub use framebuffer::{Cell, Frame};
pub use keymap::{keymap_for_mode, parse_chord, parse_chord_with_mode, LAYOUT};
pub use layout::{CELL_H, CELL_W, COLS, FB_H, FB_W, GRID_H, ROWS};
pub use render::render;
pub use sim::render_to_rgba;

/// Error type for the core crate. `Render` is reserved for M1 when the sim
/// backend may fail (e.g. font asset not found); it is unused in M0.
#[derive(Debug)]
pub enum CoreError {
    #[allow(dead_code)]
    Render(String),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Render(msg) => write!(f, "render error: {msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// M0 entry point: build the default `AppState`, render it once into a
/// `Frame` with the default theme, push that frame through the sim
/// backend, and print the title plus the output buffer size. Returns
/// `Ok(())` unconditionally for M0.
pub fn run() -> Result<(), CoreError> {
    let state = AppState::booting();
    let theme = default_theme();
    let frame = render(&state, &theme);
    let buf = render_to_rgba(&frame);
    println!("m5Tui v0.1.0");
    println!(
        "frame: {}x{} cells ({}x{} px, {} bytes RGBA8888)",
        COLS,
        ROWS,
        FB_W,
        FB_H,
        buf.len(),
    );
    Ok(())
}
