//! `m5tui-core` — the pure rendering core for the m5Tui firmware/sim.
//!
//! M0 scope: a tiny `AppState` with a pure reducer, a 40x16 framebuffer of
//! `Cell`s, a 5x8 ASCII glyph atlas, a `render` that centres the app title
//! on the screen, and a sim backend that converts a `Frame` to a 240x135
//! RGBA8888 byte buffer. No I/O, no allocations in the hot path beyond the
//! output buffer in `render_to_rgba`.

pub mod app;
pub mod event;
pub mod framebuffer;
pub mod layout;
pub mod render;
pub mod sim;

pub use app::{reduce, AppState};
pub use event::Event;
pub use framebuffer::{Cell, Frame};
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
/// `Frame`, push that frame through the sim backend, and print the title
/// plus the output buffer size. Returns `Ok(())` unconditionally for M0.
pub fn run() -> Result<(), CoreError> {
    let state = AppState::default();
    let frame = render(&state);
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
