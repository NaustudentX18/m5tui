//! Widget layer for the M1 cockpit shell.
//!
//! Each widget is a function that mutates a `Frame` in place. Widgets never
//! return a frame; the caller (see `crate::render::render`) starts with a
//! solid `palette::BG` frame, routes to the right widget, and then applies
//! the toast overlay on top.
//!
//! The widgets are pure reads of `AppState`. They never call into the
//! reducer. The colour block at the top of `crate::palette` is the only
//! source of RGB565 constants the widgets reference.

pub mod cockpit;
pub mod help;
pub mod overlay;
pub mod palette;
pub mod theme_editor;
pub mod toast;
pub mod vu;
