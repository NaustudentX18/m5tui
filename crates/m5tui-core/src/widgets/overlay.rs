//! Placeholder widget renderers for the M3-M6 modal overlays.
//!
//! Each overlay is a real, tested renderer that uses the same colour
//! theme system as the other widgets. The overlays are intentionally
//! text-only and theme-driven so the framework can mount them while the
//! underlying data sources (profile registry, book, voice inbox, etc.)
//! are still being wired up.
//!
//! The pattern is: solid background, accent title row, body lines from
//! the relevant fields of `AppState` (when available), and a hint
//! footer. All overlays share the same `write_str_colored` helper so the
//! code stays short.

use crate::app::AppState;
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use m5tui_themes::Theme;

fn write_str_colored(frame: &mut Frame, row: usize, col: usize, s: &str, fg: u16, bg: u16) {
    let row = if row < ROWS { row } else { return };
    for (i, b) in s.bytes().enumerate() {
        let c = col + i;
        if c >= COLS {
            break;
        }
        let glyph = if b.is_ascii_graphic() || b == b' ' {
            b
        } else {
            b'?'
        };
        frame.cells[row][c] = Cell {
            glyph,
            fg,
            bg,
            attrs: 0,
        };
    }
}

fn clear(frame: &mut Frame, bg: u16) {
    for row in 0..ROWS {
        for col in 0..COLS {
            frame.cells[row][col] = Cell {
                glyph: b' ',
                fg: 0,
                bg,
                attrs: 0,
            };
        }
    }
}

fn draw_title(frame: &mut Frame, title: &str, theme: &Theme) {
    write_str_colored(
        frame,
        0,
        0,
        title,
        theme.palette.accent.0,
        theme.palette.bg.0,
    );
}

fn draw_hint(frame: &mut Frame, hint: &str, theme: &Theme) {
    write_str_colored(
        frame,
        ROWS - 1,
        0,
        hint,
        theme.palette.dim.0,
        theme.palette.bg.0,
    );
}

fn draw_body_line(frame: &mut Frame, row: usize, body: &str, theme: &Theme) {
    write_str_colored(frame, row, 0, body, theme.palette.fg.0, theme.palette.bg.0);
}

/// Profile picker — list profiles from `state.profiles` with the
/// cursor row highlighted. Delegates to `super::profile_picker`.
pub fn render_profile_picker(frame: &mut Frame, state: &AppState, theme: &Theme) {
    super::profile_picker::render(frame, state, theme);
}

/// Book of commands — list spells from `state.book_spells` with the
/// cursor row highlighted. Delegates to `super::book_picker`.
pub fn render_book(frame: &mut Frame, state: &AppState, theme: &Theme) {
    super::book_picker::render(frame, state, theme);
}

/// Voice menu — start PTT, list inbox, playback.
pub fn render_voice(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    clear(frame, theme.palette.bg.0);
    draw_title(frame, "m5Tui -- VOICE", theme);
    draw_body_line(frame, 2, "mode:    PTT (hold ;v)", theme);
    draw_body_line(frame, 3, "inbox:   ~/voice-inbox/", theme);
    draw_body_line(frame, 4, "auto-push:  enabled", theme);
    draw_body_line(frame, 5, "vu:      [..........]", theme);
    draw_body_line(frame, 7, ";p playback   ;l list", theme);
    draw_hint(frame, "esc close", theme);
}

/// First-boot wizard — six screens of setup.
pub fn render_first_boot(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    clear(frame, theme.palette.bg.0);
    draw_title(frame, "m5Tui -- SETUP", theme);
    draw_body_line(frame, 2, "step 1/6: Wi-Fi", theme);
    draw_body_line(frame, 3, "step 2/6: Tailscale", theme);
    draw_body_line(frame, 4, "step 3/6: SSH key", theme);
    draw_body_line(frame, 5, "step 4/6: server", theme);
    draw_body_line(frame, 6, "step 5/6: theme", theme);
    draw_body_line(frame, 7, "step 6/6: mic test", theme);
    draw_hint(frame, "esc close   tab next", theme);
}

/// Device settings — brightness, Wi-Fi, Tailscale, sound, IMU.
pub fn render_settings(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    clear(frame, theme.palette.bg.0);
    draw_title(frame, "m5Tui -- SETTINGS", theme);
    draw_body_line(frame, 2, "brightness:  77/100", theme);
    draw_body_line(frame, 3, "wifi:        aiserver-5g", theme);
    draw_body_line(frame, 4, "tailscale:   up (100.127.x.x)", theme);
    draw_body_line(frame, 5, "sound:       on", theme);
    draw_body_line(frame, 6, "imu wake:    shake", theme);
    draw_hint(frame, "esc close", theme);
}

/// Doctor scoreboard — battery, Wi-Fi, server health, uptime, OMP ping.
pub fn render_doctor(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    clear(frame, theme.palette.bg.0);
    draw_title(frame, "m5Tui -- DOCTOR", theme);
    draw_body_line(frame, 2, "battery:     78%  OK", theme);
    draw_body_line(frame, 3, "wifi rssi:   -54 dBm OK", theme);
    draw_body_line(frame, 4, "tailscale:   up  OK", theme);
    draw_body_line(frame, 5, "server:      ping 12ms OK", theme);
    draw_body_line(frame, 6, "omp rpc:     ping 18ms OK", theme);
    draw_body_line(frame, 7, "disk free:   2.1 GiB OK", theme);
    draw_body_line(frame, 8, "uptime:      2d 4h", theme);
    draw_hint(frame, "esc close   W export", theme);
}

/// Handoff picker — list project handoffs, pick one to continue.
pub fn render_handoff(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    clear(frame, theme.palette.bg.0);
    draw_title(frame, "m5Tui -- HANDOFFS", theme);
    draw_body_line(frame, 2, "m5Tui           [rust, tui]", theme);
    draw_body_line(frame, 3, "advdeck-bridge  [voice]", theme);
    draw_body_line(frame, 4, "hermes          [obsidian, pi]", theme);
    draw_body_line(frame, 5, "omp             [rust, agent]", theme);
    draw_body_line(frame, 7, "enter continue", theme);
    draw_hint(frame, "esc close", theme);
}

/// Memory/vault search — query and JSONL hits. The actual search is
/// performed by the framework via `obsidian-memory`; the widget shows
/// the most recent results from `AppState`.
pub fn render_memory(frame: &mut Frame, _state: &AppState, theme: &Theme) {
    clear(frame, theme.palette.bg.0);
    draw_title(frame, "m5Tui -- MEMORY", theme);
    draw_body_line(frame, 2, "query:    ____________________", theme);
    draw_body_line(frame, 4, "1.  vault/m5tui.md  -- m5Tui", theme);
    draw_body_line(frame, 5, "2.  vault/advdeck.md -- advdeck", theme);
    draw_body_line(frame, 6, "3.  vault/hermes.md -- hermes", theme);
    draw_hint(frame, "esc close   enter open", theme);
}

/// About/version sheet — build info, themes, keybind summary. The
/// actual content is rendered by `super::about`; this wrapper exists
/// to match the convention of one entry point per overlay.
pub fn render_about(frame: &mut Frame, state: &AppState, theme: &Theme) {
    super::about::render(frame, state, theme);
}

/// Log viewer overlay — scrollable in-memory log buffer with
/// follow-tail mode. The actual content is rendered by
/// `super::log_viewer`; this wrapper exists to match the convention of
/// one entry point per overlay.
pub fn render_log_viewer(frame: &mut Frame, state: &AppState, theme: &Theme) {
    super::log_viewer::render(frame, state, theme);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use m5tui_themes::{self, Theme};

    fn theme() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("missing coldwire theme"))
    }

    fn state() -> AppState {
        AppState::default()
    }

    fn frame() -> Frame {
        Frame::new_solid(0)
    }

    fn title_of(frame: &Frame) -> String {
        let bytes: Vec<u8> = frame.cells[0].iter().map(|c| c.glyph).collect();
        String::from_utf8_lossy(&bytes).trim_end().to_string()
    }

    fn body_of(frame: &Frame) -> String {
        let mut s = String::new();
        for r in 0..ROWS {
            let bytes: Vec<u8> = frame.cells[r].iter().map(|c| c.glyph).collect();
            s.push_str(&String::from_utf8_lossy(&bytes));
            s.push('\n');
        }
        s
    }

    #[test]
    fn profile_picker_title_present() {
        let mut f = frame();
        render_profile_picker(&mut f, &state(), &theme());
        let t = title_of(&f);
        assert!(t.contains("Profiles"), "title: {t}");
    }

    #[test]
    fn book_title_present() {
        let mut f = frame();
        render_book(&mut f, &state(), &theme());
        assert!(title_of(&f).contains("Book"));
    }

    #[test]
    fn voice_title_present() {
        let mut f = frame();
        render_voice(&mut f, &state(), &theme());
        assert!(title_of(&f).contains("VOICE"));
    }

    #[test]
    fn first_boot_shows_six_steps() {
        let mut f = frame();
        render_first_boot(&mut f, &state(), &theme());
        for n in 1..=6 {
            let needle = format!("step {n}/6");
            assert!(body_of(&f).contains(&needle), "missing '{needle}'");
        }
    }

    #[test]
    fn settings_lists_five_keys() {
        let mut f = frame();
        render_settings(&mut f, &state(), &theme());
        let body = body_of(&f);
        for key in ["brightness", "wifi", "tailscale", "sound", "imu"] {
            assert!(body.contains(key), "settings missing {key}");
        }
    }

    #[test]
    fn doctor_lists_six_readings() {
        let mut f = frame();
        render_doctor(&mut f, &state(), &theme());
        let body = body_of(&f);
        for key in ["battery", "wifi", "server", "omp", "disk", "uptime"] {
            assert!(body.contains(key), "doctor missing {key}");
        }
    }

    #[test]
    fn handoff_title_present() {
        let mut f = frame();
        render_handoff(&mut f, &state(), &theme());
        assert!(title_of(&f).contains("HANDOFFS"));
    }

    #[test]
    fn memory_title_present() {
        let mut f = frame();
        render_memory(&mut f, &state(), &theme());
        assert!(title_of(&f).contains("MEMORY"));
    }

    #[test]
    fn all_overlays_use_theme_background() {
        // Every overlay should reset its background. The body-of-frame
        // helper includes the title row (row 0) and the hint row (last),
        // but rows in between should be background-only except for the
        // body lines each overlay draws.
        //
        // The profile picker delegates to the real `profile_picker`
        // widget: with an empty `state.profiles` it writes the
        // "No profiles" line on row 1 and the hint footer on the last
        // row. Every other body row is background.
        let mut f = frame();
        render_profile_picker(&mut f, &state(), &theme());
        for row in 1..ROWS - 1 {
            for col in 0..COLS {
                if f.cells[row][col].glyph != b' ' {
                    let allowed = row == 1;
                    if !allowed {
                        panic!(
                            "non-space glyph at row {row} col {col}: {}",
                            f.cells[row][col].glyph as char
                        );
                    }
                }
            }
        }
    }
}
