//! The M1 cockpit (home screen).
//!
//! Layout (40x16):
//!
//! ```text
//!   0:  aiserver-1 :: OK :: up 4h12m :: 73% bat      (top bar, full row)
//!   1:  AGENTS                SESSION                 (headers, row 1)
//!   2:  > aiserver-1/omp ok   model MiniMax-M3
//!   3:    aiserver-1/bridge   task step 3/9 in_flt
//!   4:    pc-ollama/midimax   rate 31.4 t/s
//!   5:    pc-tailscale   warn  ctx 12K/512K
//!   6:    jbpi/bridge   down
//!   7-12: (empty / future)
//!  13:  ;/ palette | ;v voice | ;t theme | ;? help   (hint bar)
//!  14:  --------------------------------------------   (separator)
//!  15:  > {prompt}_                                  (prompt line)
//! ```
//!
//! The selected agent's row is rendered in `palette::ACCENT` with a `>`
//! cursor in column 0. The `down` status uses `palette::DIM`; everything
//! else is `palette::FG` on `palette::BG`.

use crate::app::{AppState, MockAgent, MockSession};
use crate::framebuffer::{Cell, Frame};
use crate::layout::{COLS, ROWS};
use crate::palette;

/// Draw the cockpit into the given frame. Idempotent and pure: it reads
/// `state` and mutates `frame`; no allocation, no I/O.
pub fn render(frame: &mut Frame, state: &AppState) {
    draw_top_bar(frame);
    draw_headers(frame);
    draw_agents(frame, &state.agents, state.selected_agent);
    draw_session(frame, &state.session);
    draw_hint_bar(frame);
    draw_separator(frame);
    draw_prompt(frame, &state.prompt);
}

fn draw_top_bar(frame: &mut Frame) {
    // 40 chars: "aiserver-1 :: OK :: up 4h12m :: 73% bat "
    let bar = "aiserver-1 :: OK :: up 4h12m :: 73% bat ";
    debug_assert_eq!(bar.len(), COLS);
    for (col, b) in bar.bytes().enumerate() {
        frame.cells[0][col] = Cell {
            glyph: b,
            fg: palette::FG,
            bg: palette::BG,
            attrs: 0,
        };
    }
    // Highlight the OK marker and battery percent in ACCENT.
    write_str_colored(frame, 0, 13, "OK", palette::ACCENT, palette::BG);
    write_str_colored(frame, 0, 33, "73", palette::ACCENT, palette::BG);
}

fn draw_headers(frame: &mut Frame) {
    write_str_colored(frame, 1, 0, "AGENTS", palette::ACCENT, palette::BG);
    write_str_colored(frame, 1, 20, "SESSION", palette::ACCENT, palette::BG);
}

fn draw_agents(frame: &mut Frame, agents: &[MockAgent], selected: usize) {
    for (i, agent) in agents.iter().take(5).enumerate() {
        let row = 2 + i;
        if row >= 13 {
            break;
        }
        let is_selected = i == selected;
        let cursor = if is_selected { b'>' } else { b' ' };
        let fg = if is_selected {
            palette::ACCENT
        } else {
            palette::FG
        };
        let status_fg = match (is_selected, agent.status.as_str()) {
            (_, "down") => palette::DIM,
            (true, _) => palette::ACCENT,
            (false, _) => palette::FG,
        };
        frame.cells[row][0] = Cell {
            glyph: cursor,
            fg,
            bg: palette::BG,
            attrs: 0,
        };
        // 18-char name (truncate or pad with spaces).
        let name = pad_or_truncate(&agent.name, 18);
        write_str_colored(frame, row, 1, &name, fg, palette::BG);
        // 5-char status (uppercased).
        let status = pad_or_truncate(&agent.status.to_uppercase(), 5);
        write_str_colored(frame, row, 19, &status, status_fg, palette::BG);
    }
}

fn draw_session(frame: &mut Frame, session: &MockSession) {
    // Each label is rendered in FG, each value in ACCENT.
    // Row 2: model <name>
    write_str_colored(frame, 2, 20, "model", palette::FG, palette::BG);
    write_str_colored(frame, 2, 26, &session.model, palette::ACCENT, palette::BG);
    // Row 3: task N/M in_flt
    let task_str = format!("task {}/{} in_flt", session.in_flight, session.total);
    write_str_colored(frame, 3, 20, &task_str, palette::FG, palette::BG);
    // Row 4: rate N.N t/s
    let rate_str = format!("rate {:.1} t/s", session.rate_tps);
    write_str_colored(frame, 4, 20, &rate_str, palette::FG, palette::BG);
    // Row 5: ctx N/M
    let ctx_str = format!(
        "ctx {}/{}",
        format_kb(session.ctx_used),
        format_kb(session.ctx_max),
    );
    write_str_colored(frame, 5, 20, &ctx_str, palette::FG, palette::BG);
}

fn draw_hint_bar(frame: &mut Frame) {
    let hint = ";/ palette | ;v voice | ;t theme | ;? help";
    write_str_colored(frame, 13, 0, hint, palette::FG, palette::BG);
}

fn draw_separator(frame: &mut Frame) {
    for col in 0..COLS {
        frame.cells[14][col] = Cell {
            glyph: b'-',
            fg: palette::DIM,
            bg: palette::BG,
            attrs: 0,
        };
    }
}

fn draw_prompt(frame: &mut Frame, prompt: &str) {
    // Row 15: "> {prompt}_" with a cursor. The `_` is drawn at the end of
    // the prompt, which stands in for a real caret glyph (the real device
    // blinks a hardware cursor at this position).
    write_str_colored(frame, 15, 0, "> ", palette::ACCENT, palette::BG);
    let rest = &prompt[..prompt.len().min(38)];
    write_str_colored(frame, 15, 2, rest, palette::FG, palette::BG);
    let cursor_col = 2 + rest.len();
    if cursor_col < COLS {
        frame.cells[15][cursor_col] = Cell {
            glyph: b'_',
            fg: palette::ACCENT,
            bg: palette::BG,
            attrs: 0,
        };
    }
}

/// Write a string into the frame at `(row, col)`, clipped at `COLS`. Bytes
/// are taken as ASCII; non-ASCII bytes are rendered as `?`.
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

/// Truncate `s` to `max` bytes (no Unicode handling — input is ASCII) and
/// right-pad with spaces to `max` bytes.
fn pad_or_truncate(s: &str, max: usize) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= max {
        String::from_utf8(bytes[..max].to_vec()).unwrap_or_default()
    } else {
        let mut out = String::from(s);
        for _ in bytes.len()..max {
            out.push(' ');
        }
        out
    }
}

/// Format an integer byte count as a short human-readable value: 12300 ->
/// "12K", 512000 -> "512K". M1 just uses K-suffixed values for both
/// numerator and denominator.
fn format_kb(n: u32) -> String {
    if n >= 1024 {
        format!("{}K", n / 1024)
    } else {
        format!("{}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Mode;
    use crate::event::Focus;

    fn cockpit_state() -> AppState {
        AppState {
            mode: Mode::Cockpit,
            focus: Focus::Agents,
            ..AppState::default()
        }
    }

    #[test]
    fn cockpit_renders_top_bar_with_server_status() {
        let mut f = Frame::new_solid(palette::BG);
        let s = cockpit_state();
        render(&mut f, &s);
        // The top bar begins with `aiserver-1 :: OK ...`. The 'a' at
        // (row=0, col=0) is the first byte.
        assert_eq!(f.cells[0][0].glyph, b'a');
        assert_eq!(f.cells[0][10].glyph, b' ');
        assert_eq!(f.cells[0][11].glyph, b':');
        // The "OK" marker at (row=0, col=13..15) is drawn in ACCENT.
        assert_eq!(f.cells[0][13].glyph, b'O');
        assert_eq!(f.cells[0][13].fg, palette::ACCENT);
        assert_eq!(f.cells[0][14].glyph, b'K');
    }

    #[test]
    fn cockpit_renders_agent_list_with_cursor_on_selected() {
        let mut f = Frame::new_solid(palette::BG);
        let s = cockpit_state();
        render(&mut f, &s);
        // The first agent `aiserver-1/omp` is at (row=2, col=1). The '>'
        // cursor is at (row=2, col=0) in ACCENT.
        assert_eq!(f.cells[2][0].glyph, b'>');
        assert_eq!(f.cells[2][0].fg, palette::ACCENT);
        assert_eq!(f.cells[2][1].glyph, b'a');
        assert_eq!(f.cells[2][2].glyph, b'i');
        // The 5th agent (jbpi/bridge) has status `down` -> DIM, and is
        // not selected -> FG for the name + cursor space.
        let row = 2 + 4;
        assert_eq!(f.cells[row][0].glyph, b' ');
        assert_eq!(f.cells[row][0].fg, palette::FG);
        // Status at col 19..24 should be `DOWN` in DIM.
        assert_eq!(f.cells[row][19].glyph, b'D');
        assert_eq!(f.cells[row][19].fg, palette::DIM);
    }

    #[test]
    fn cockpit_renders_session_state_on_right_pane() {
        let mut f = Frame::new_solid(palette::BG);
        let s = cockpit_state();
        render(&mut f, &s);
        // The session block starts at col 20 on row 2 with "model".
        assert_eq!(f.cells[2][20].glyph, b'm');
        assert_eq!(f.cells[2][20].fg, palette::FG);
        // The model name (MiniMax-M3) starts at col 26 in ACCENT.
        assert_eq!(f.cells[2][26].glyph, b'M');
        assert_eq!(f.cells[2][26].fg, palette::ACCENT);
        // The rate row has `rate` at col 20.
        assert_eq!(f.cells[4][20].glyph, b'r');
        assert_eq!(f.cells[4][25].glyph, b'3');
    }

    #[test]
    fn cockpit_renders_hint_bar_and_separator() {
        let mut f = Frame::new_solid(palette::BG);
        let s = cockpit_state();
        render(&mut f, &s);
        // Hint bar at row 13 starts with `;/`.
        assert_eq!(f.cells[13][0].glyph, b';');
        assert_eq!(f.cells[13][1].glyph, b'/');
        // Separator at row 14 is all `-` in DIM.
        for col in 0..COLS {
            assert_eq!(f.cells[14][col].glyph, b'-');
            assert_eq!(f.cells[14][col].fg, palette::DIM);
        }
    }

    #[test]
    fn cockpit_renders_prompt_with_cursor() {
        let mut f = Frame::new_solid(palette::BG);
        let s = AppState {
            prompt: "hello".into(),
            ..cockpit_state()
        };
        render(&mut f, &s);
        // Row 15: "> hello_" with the `>` in ACCENT and the `_` caret at
        // col 2 + 5 = 7.
        assert_eq!(f.cells[15][0].glyph, b'>');
        assert_eq!(f.cells[15][0].fg, palette::ACCENT);
        assert_eq!(f.cells[15][2].glyph, b'h');
        assert_eq!(f.cells[15][6].glyph, b'o');
        assert_eq!(f.cells[15][7].glyph, b'_');
        assert_eq!(f.cells[15][7].fg, palette::ACCENT);
    }

    #[test]
    fn cockpit_prompt_cursor_at_start_when_empty() {
        let mut f = Frame::new_solid(palette::BG);
        let s = cockpit_state();
        render(&mut f, &s);
        // Empty prompt: cursor `_` at col 2.
        assert_eq!(f.cells[15][2].glyph, b'_');
    }
}
