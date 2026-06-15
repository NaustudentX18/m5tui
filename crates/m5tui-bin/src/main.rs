//! m5Tui binary shim. All logic lives in `m5tui-core`.
//!
//! For M0 this is a thin wrapper that calls `m5tui_core::run()`.
//! Later milestones will add arg parsing (profile picker, theme
//! override, doctor mode, etc.) here.

fn main() {
    if let Err(e) = m5tui_core::run() {
        eprintln!("m5Tui: {e}");
        std::process::exit(1);
    }
}
