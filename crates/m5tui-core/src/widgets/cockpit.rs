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
use m5tui_themes::Theme;

/// Draw the cockpit into the given frame. Idempotent and pure: it reads
/// `state` and mutates `frame`; no allocation, no I/O. M2: the widget
/// is theme-aware — all colors come from `theme.palette`, not the
/// hardcoded `crate::palette` constants.
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    draw_top_bar(frame, theme);
    draw_headers(frame, theme);
    draw_agents(frame, &state.agents, state.selected_agent, theme);
    // The right pane is owned by the OMP cards widget when the
    // session is producing tool calls / thinking / todos; otherwise
    // it falls back to the static `MockSession` view.
    if state.omp_cards.is_empty() && state.omp_thinking.is_none() && state.omp_todos.is_empty() {
        draw_session(frame, &state.session, theme);
    } else {
        crate::widgets::omp_cards::render(frame, state, theme);
    }
    draw_hint_bar(frame, theme);
    draw_separator(frame, theme);
    draw_prompt(frame, &state.prompt, theme);
}

fn draw_top_bar(frame: &mut Frame, theme: &Theme) {
    // 40 chars: "aiserver-1 :: OK :: up 4h12m :: 73% bat "
    let bar = "aiserver-1 :: OK :: up 4h12m :: 73% bat ";
    debug_assert_eq!(bar.len(), COLS);
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    for (col, b) in bar.bytes().enumerate() {
        frame.cells[0][col] = Cell {
            glyph: b,
            fg,
            bg,
            attrs: 0,
        };
    }
    // Highlight the OK marker and battery percent in the theme accent.
    write_str_colored(frame, 0, 13, "OK", accent, bg);
    write_str_colored(frame, 0, 33, "73", accent, bg);
}

fn draw_headers(frame: &mut Frame, theme: &Theme) {
    let accent = theme.palette.accent.0;
    let bg = theme.palette.bg.0;
    write_str_colored(frame, 1, 0, "AGENTS", accent, bg);
    write_str_colored(frame, 1, 20, "SESSION", accent, bg);
}

fn draw_agents(frame: &mut Frame, agents: &[MockAgent], selected: usize, theme: &Theme) {
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    let dim = theme.palette.dim.0;
    for (i, agent) in agents.iter().take(5).enumerate() {
        let row = 2 + i;
        if row >= 13 {
            break;
        }
        let is_selected = i == selected;
        let cursor = if is_selected { b'>' } else { b' ' };
        let name_fg = if is_selected { accent } else { fg };
        let status_fg = match (is_selected, agent.status.as_str()) {
            (_, "down") => dim,
            (true, _) => accent,
            (false, _) => fg,
        };
        frame.cells[row][0] = Cell {
            glyph: cursor,
            fg: name_fg,
            bg,
            attrs: 0,
        };
        // 18-char name (truncate or pad with spaces).
        let name = pad_or_truncate(&agent.name, 18);
        write_str_colored(frame, row, 1, &name, name_fg, bg);
        // 5-char status (uppercased).
        let status = pad_or_truncate(&agent.status.to_uppercase(), 5);
        write_str_colored(frame, row, 19, &status, status_fg, bg);
    }
}

fn draw_session(frame: &mut Frame, session: &MockSession, theme: &Theme) {
    // Each label is rendered in FG, each value in the theme's accent.
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    // Row 2: model <name>
    write_str_colored(frame, 2, 20, "model", fg, bg);
    write_str_colored(frame, 2, 26, &session.model, accent, bg);
    // Row 3: task N/M in_flt
    let task_str = format!("task {}/{} in_flt", session.in_flight, session.total);
    write_str_colored(frame, 3, 20, &task_str, fg, bg);
    // Row 4: rate N.N t/s
    let rate_str = format!("rate {:.1} t/s", session.rate_tps);
    write_str_colored(frame, 4, 20, &rate_str, fg, bg);
    // Row 5: ctx N/M
    let ctx_str = format!(
        "ctx {}/{}",
        format_kb(session.ctx_used),
        format_kb(session.ctx_max),
    );
    write_str_colored(frame, 5, 20, &ctx_str, fg, bg);
}

fn draw_hint_bar(frame: &mut Frame, theme: &Theme) {
    let hint = ";/ palette | ;v voice | ;t theme | ;? help";
    write_str_colored(frame, 13, 0, hint, theme.palette.fg.0, theme.palette.bg.0);
}

fn draw_separator(frame: &mut Frame, theme: &Theme) {
    for col in 0..COLS {
        frame.cells[14][col] = Cell {
            glyph: b'-',
            fg: theme.palette.dim.0,
            bg: theme.palette.bg.0,
            attrs: 0,
        };
    }
}

fn draw_prompt(frame: &mut Frame, prompt: &str, theme: &Theme) {
    // Row 15: "> {prompt}_" with a cursor. The `_` is drawn at the end of
    // the prompt, which stands in for a real caret glyph (the real device
    // blinks a hardware cursor at this position).
    let fg = theme.palette.fg.0;
    let bg = theme.palette.bg.0;
    let accent = theme.palette.accent.0;
    write_str_colored(frame, 15, 0, "> ", accent, bg);
    let rest = &prompt[..prompt.len().min(38)];
    write_str_colored(frame, 15, 2, rest, fg, bg);
    let cursor_col = 2 + rest.len();
    if cursor_col < COLS {
        frame.cells[15][cursor_col] = Cell {
            glyph: b'_',
            fg: accent,
            bg,
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

    fn coldwire() -> Theme {
        m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire builtin missing"))
    }

    fn cockpit_state() -> AppState {
        AppState {
            mode: Mode::Cockpit,
            focus: Focus::Agents,
            ..AppState::default()
        }
    }

    #[test]
    fn cockpit_renders_top_bar_with_server_status() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = cockpit_state();
        render(&mut f, &s, &theme);
        // The top bar begins with `aiserver-1 :: OK ...`. The 'a' at
        // (row=0, col=0) is the first byte.
        assert_eq!(f.cells[0][0].glyph, b'a');
        assert_eq!(f.cells[0][10].glyph, b' ');
        assert_eq!(f.cells[0][11].glyph, b':');
        // The "OK" marker at (row=0, col=13..15) is drawn in the theme accent.
        assert_eq!(f.cells[0][13].glyph, b'O');
        assert_eq!(f.cells[0][13].fg, theme.palette.accent.0);
        assert_eq!(f.cells[0][14].glyph, b'K');
    }

    #[test]
    fn cockpit_renders_agent_list_with_cursor_on_selected() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = cockpit_state();
        render(&mut f, &s, &theme);
        // The first agent `aiserver-1/omp` is at (row=2, col=1). The '>'
        // cursor is at (row=2, col=0) in the theme accent.
        assert_eq!(f.cells[2][0].glyph, b'>');
        assert_eq!(f.cells[2][0].fg, theme.palette.accent.0);
        assert_eq!(f.cells[2][1].glyph, b'a');
        assert_eq!(f.cells[2][2].glyph, b'i');
        // The 5th agent (jbpi/bridge) has status `down` -> DIM, and is
        // not selected -> FG for the name + cursor space.
        let row = 2 + 4;
        assert_eq!(f.cells[row][0].glyph, b' ');
        assert_eq!(f.cells[row][0].fg, theme.palette.fg.0);
        // Status at col 19..24 should be `DOWN` in DIM.
        assert_eq!(f.cells[row][19].glyph, b'D');
        assert_eq!(f.cells[row][19].fg, theme.palette.dim.0);
    }

    #[test]
    fn cockpit_renders_session_state_on_right_pane() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = cockpit_state();
        render(&mut f, &s, &theme);
        // The session block starts at col 20 on row 2 with "model".
        assert_eq!(f.cells[2][20].glyph, b'm');
        assert_eq!(f.cells[2][20].fg, theme.palette.fg.0);
        // The model name (MiniMax-M3) starts at col 26 in the theme accent.
        assert_eq!(f.cells[2][26].glyph, b'M');
        assert_eq!(f.cells[2][26].fg, theme.palette.accent.0);
        // The rate row has `rate` at col 20.
        assert_eq!(f.cells[4][20].glyph, b'r');
        assert_eq!(f.cells[4][25].glyph, b'3');
    }

    #[test]
    fn cockpit_renders_hint_bar_and_separator() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = cockpit_state();
        render(&mut f, &s, &theme);
        // Hint bar at row 13 starts with `;/`.
        assert_eq!(f.cells[13][0].glyph, b';');
        assert_eq!(f.cells[13][1].glyph, b'/');
        // Separator at row 14 is all `-` in DIM.
        for col in 0..COLS {
            assert_eq!(f.cells[14][col].glyph, b'-');
            assert_eq!(f.cells[14][col].fg, theme.palette.dim.0);
        }
    }

    #[test]
    fn cockpit_renders_prompt_with_cursor() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = AppState {
            prompt: "hello".into(),
            ..cockpit_state()
        };
        render(&mut f, &s, &theme);
        // Row 15: "> hello_" with the `>` in accent and the `_` caret at
        // col 2 + 5 = 7.
        assert_eq!(f.cells[15][0].glyph, b'>');
        assert_eq!(f.cells[15][0].fg, theme.palette.accent.0);
        assert_eq!(f.cells[15][2].glyph, b'h');
        assert_eq!(f.cells[15][6].glyph, b'o');
        assert_eq!(f.cells[15][7].glyph, b'_');
        assert_eq!(f.cells[15][7].fg, theme.palette.accent.0);
    }

    #[test]
    fn cockpit_prompt_cursor_at_start_when_empty() {
        let theme = coldwire();
        let mut f = Frame::new_solid(theme.palette.bg.0);
        let s = cockpit_state();
        render(&mut f, &s, &theme);
        // Empty prompt: cursor `_` at col 2.
        assert_eq!(f.cells[15][2].glyph, b'_');
    }
}
