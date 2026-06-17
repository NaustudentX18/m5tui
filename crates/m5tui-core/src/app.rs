//! Pure application state. The reducer is `Event -> AppState -> AppState`.
//!
//! M1 extends `AppState` with the home-screen fields (mode, focus, agents,
//! session, prompt, palette state, clock, toast) and adds `step`, the
//! side-channel reducer that also returns the `Outgoing` actions the
//! framework must perform.

use crate::event::{Event, Focus, KeyAction, Outgoing};

/// Which top-level screen the user is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Home screen: top bar, agent list, session state, prompt, toast.
    Cockpit,
    /// Command palette modal: 12 builtin commands + fuzzy search.
    Palette,
    /// Hotkey/help overlay.
    Help,
    /// On-device theme editor overlay (9 screens).
    ThemeEditor,
    /// Profile picker modal — list profiles, pick one to connect.
    ProfilePicker,
    /// Book of commands modal — list spells, pick one to run.
    Book,
    /// Voice menu — list recordings, start a new PTT capture, etc.
    Voice,
    /// First-boot wizard — Wi-Fi/Tailscale/SSH key/server/theme/mic.
    FirstBoot,
    /// Device settings screen — brightness, Wi-Fi, Tailscale, sound, IMU.
    Settings,
    /// Doctor scoreboard modal — battery/Wi-Fi/server health summary.
    Doctor,
    /// Handoff picker modal — list project handoffs, pick one to continue.
    Handoff,
    /// Memory/vault search modal — query and JSONL hits.
    Memory,
}

/// A mock agent shown in the cockpit's left pane. Real agents arrive in
/// M3; in M1 the reducer carries five of these so the layout has data to
/// render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockAgent {
    pub name: String,
    pub kind: String,
    pub status: String,
}

/// A mock OMP session shown in the cockpit's right pane. Real session
/// data arrives in M4.
#[derive(Debug, Clone, PartialEq)]
pub struct MockSession {
    pub model: String,
    pub task: String,
    pub in_flight: u32,
    pub total: u32,
    pub rate_tps: f32,
    pub ctx_used: u32,
    pub ctx_max: u32,
}

/// A short-lived message shown in the top-right of the cockpit. Toasts
/// expire when `clock >= until_tick`; the reducer drops them on `Tick`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub text: String,
    pub until_tick: u64,
}

/// The full application state. The reducer is the only writer; the
/// renderer is a pure read.
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub title: String,
    pub frame_count: u64,
    pub should_quit: bool,

    pub mode: Mode,
    pub focus: Focus,
    pub agents: Vec<MockAgent>,
    pub selected_agent: usize,
    pub session: MockSession,
    pub prompt: String,
    pub palette_query: String,
    pub palette_selected: usize,
    pub clock: u64,
    pub toast: Option<Toast>,
}

impl Default for AppState {
    fn default() -> Self {
        let agents = vec![
            MockAgent {
                name: "aiserver-1/omp".into(),
                kind: "omp".into(),
                status: "ok".into(),
            },
            MockAgent {
                name: "aiserver-1/bridge".into(),
                kind: "bridge".into(),
                status: "ok".into(),
            },
            MockAgent {
                name: "pc-ollama/midimax".into(),
                kind: "ollama".into(),
                status: "ok".into(),
            },
            MockAgent {
                name: "pc-tailscale".into(),
                kind: "tailscale".into(),
                status: "warn".into(),
            },
            MockAgent {
                name: "jbpi/bridge".into(),
                kind: "bridge".into(),
                status: "down".into(),
            },
        ];
        let session = MockSession {
            model: "MiniMax-M3".into(),
            task: "step 3/9 -- refactor".into(),
            in_flight: 2,
            total: 9,
            rate_tps: 31.4,
            ctx_used: 12_300,
            ctx_max: 512_000,
        };
        Self {
            title: "m5Tui v0.1.0".into(),
            frame_count: 0,
            should_quit: false,
            mode: Mode::Cockpit,
            focus: Focus::Prompt,
            agents,
            selected_agent: 0,
            session,
            prompt: String::new(),
            palette_query: String::new(),
            palette_selected: 0,
            clock: 0,
            toast: None,
        }
    }
}

/// Pure reducer (M0 signature kept). Handles only the events that have no
/// outgoing side effect. `Key` and `Outgoing` events route through
/// `step` instead, which returns the actions the framework must run.
pub fn reduce(state: AppState, event: Event) -> AppState {
    match event {
        Event::Tick => AppState {
            frame_count: state.frame_count + 1,
            clock: state.clock + 1,
            toast: drop_expired_toast(&state),
            ..state
        },
        Event::Quit => AppState {
            should_quit: true,
            ..state
        },
        // Key, Outgoing, and CloseOverlay are handled by `step`; the
        // pure projection just bumps the clock so toasts still expire
        // even when the framework skips the side channel.
        Event::Key(_) | Event::Outgoing(_) | Event::CloseOverlay => AppState {
            clock: state.clock + 1,
            ..state
        },
    }
}

fn drop_expired_toast(state: &AppState) -> Option<Toast> {
    match &state.toast {
        Some(t) if state.clock >= t.until_tick => None,
        Some(_) => state.toast.clone(),
        None => None,
    }
}

/// Side-channel reducer: returns the next state and the `Outgoing`
/// actions the framework must perform. The reducer is still pure — no
/// I/O happens here, it just produces a value the framework consumes.
pub fn step(state: AppState, event: Event) -> (AppState, Vec<Outgoing>) {
    let mut out: Vec<Outgoing> = Vec::new();
    let next = match event {
        Event::Tick => {
            // Drop the toast if it expired this tick.
            let toast = match &state.toast {
                Some(t) if state.clock >= t.until_tick => None,
                Some(t) => Some(t.clone()),
                None => None,
            };
            AppState {
                clock: state.clock + 1,
                toast,
                ..state
            }
        }
        Event::Quit => {
            out.push(Outgoing::Quit);
            AppState {
                should_quit: true,
                ..state
            }
        }
        Event::Key(action) => apply_key(state, action, &mut out),
        Event::Outgoing(o) => apply_outgoing(state, o, &mut out),
        Event::CloseOverlay => AppState {
            mode: Mode::Cockpit,
            ..state
        },
    };
    (next, out)
}
/// Open a modal overlay. Returns the input state unchanged when the
/// current mode is not `Cockpit` (one overlay at a time).
fn open_overlay(state: AppState, mode: Mode, event: Outgoing, out: &mut Vec<Outgoing>) -> AppState {
    if state.mode != Mode::Cockpit {
        return state;
    }
    out.push(event);
    AppState { mode, ..state }
}

/// Save the prompt contents as a memo note. Empty prompts are ignored.
fn save_memo(state: AppState, out: &mut Vec<Outgoing>) -> AppState {
    if state.mode != Mode::Cockpit || state.prompt.is_empty() {
        return state;
    }
    out.push(Outgoing::SaveMemo(state.prompt.clone()));
    AppState {
        prompt: String::new(),
        ..state
    }
}

fn apply_key(state: AppState, action: KeyAction, out: &mut Vec<Outgoing>) -> AppState {
    match action {
        KeyAction::Palette => open_overlay(state, Mode::Palette, Outgoing::OpenPalette, out),
        KeyAction::Help => open_overlay(state, Mode::Help, Outgoing::OpenHelp, out),
        KeyAction::OpenThemeEditor => {
            open_overlay(state, Mode::ThemeEditor, Outgoing::OpenHelp, out)
        }
        KeyAction::OpenProfilePicker => {
            open_overlay(state, Mode::ProfilePicker, Outgoing::OpenProfilePicker, out)
        }
        KeyAction::OpenBook => open_overlay(state, Mode::Book, Outgoing::OpenBook, out),
        KeyAction::OpenVoice => open_overlay(state, Mode::Voice, Outgoing::OpenVoice, out),
        KeyAction::OpenFirstBoot => {
            open_overlay(state, Mode::FirstBoot, Outgoing::OpenFirstBoot, out)
        }
        KeyAction::OpenSettings => open_overlay(state, Mode::Settings, Outgoing::OpenSettings, out),
        KeyAction::RunDoctor => open_overlay(state, Mode::Doctor, Outgoing::RunDoctor, out),
        KeyAction::OpenHandoff => open_overlay(state, Mode::Handoff, Outgoing::OpenHandoff, out),
        KeyAction::OpenMemory => open_overlay(state, Mode::Memory, Outgoing::OpenMemory, out),
        KeyAction::SaveMemo => save_memo(state, out),
        KeyAction::Esc => match state.mode {
            Mode::Cockpit => AppState {
                focus: Focus::Prompt,
                ..state
            },
            _ => AppState {
                mode: Mode::Cockpit,
                ..state
            },
        },
        KeyAction::Tab => {
            let next_focus = match state.focus {
                Focus::Prompt => Focus::Agents,
                Focus::Agents => Focus::Session,
                Focus::Session => Focus::Prompt,
            };
            out.push(Outgoing::CycleFocus(next_focus));
            AppState {
                focus: next_focus,
                ..state
            }
        }
        KeyAction::Up => {
            if state.focus == Focus::Agents {
                AppState {
                    selected_agent: state.selected_agent.saturating_sub(1),
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::Down => {
            if state.focus == Focus::Agents {
                let max = state.agents.len().saturating_sub(1);
                let next = if state.selected_agent >= max {
                    max
                } else {
                    state.selected_agent + 1
                };
                AppState {
                    selected_agent: next,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::Enter => match state.focus {
            Focus::Agents => {
                out.push(Outgoing::SelectAgent(state.selected_agent));
                state
            }
            Focus::Prompt => {
                if state.prompt.is_empty() {
                    state
                } else {
                    out.push(Outgoing::SubmitPrompt(state.prompt.clone()));
                    AppState {
                        prompt: String::new(),
                        ..state
                    }
                }
            }
            Focus::Session => state,
        },
        KeyAction::Backspace => {
            if state.focus == Focus::Prompt {
                let mut p = state.prompt;
                p.pop();
                AppState { prompt: p, ..state }
            } else {
                state
            }
        }
        KeyAction::Char(c) => {
            if state.focus == Focus::Prompt {
                let mut p = state.prompt;
                p.push(c);
                AppState { prompt: p, ..state }
            } else {
                state
            }
        }
        KeyAction::Left | KeyAction::Right => state,
    }
}

fn apply_outgoing(state: AppState, o: Outgoing, out: &mut Vec<Outgoing>) -> AppState {
    match o {
        Outgoing::CloseOverlay => AppState {
            mode: Mode::Cockpit,
            ..state
        },
        Outgoing::OpenPalette => AppState {
            mode: Mode::Palette,
            palette_query: String::new(),
            palette_selected: 0,
            ..state
        },
        Outgoing::OpenHelp => AppState {
            mode: Mode::Help,
            ..state
        },
        Outgoing::OpenProfilePicker => AppState {
            mode: Mode::ProfilePicker,
            ..state
        },
        Outgoing::OpenBook => AppState {
            mode: Mode::Book,
            ..state
        },
        Outgoing::OpenVoice => AppState {
            mode: Mode::Voice,
            ..state
        },
        Outgoing::OpenFirstBoot => AppState {
            mode: Mode::FirstBoot,
            ..state
        },
        Outgoing::OpenSettings => AppState {
            mode: Mode::Settings,
            ..state
        },
        Outgoing::RunDoctor => AppState {
            mode: Mode::Doctor,
            ..state
        },
        Outgoing::OpenHandoff => AppState {
            mode: Mode::Handoff,
            ..state
        },
        Outgoing::OpenMemory => AppState {
            mode: Mode::Memory,
            ..state
        },
        Outgoing::Quit => AppState {
            should_quit: true,
            ..state
        },
        Outgoing::SubmitPrompt(_)
        | Outgoing::SelectAgent(_)
        | Outgoing::CycleFocus(_)
        | Outgoing::SaveMemo(_)
        | Outgoing::PickProfile(_)
        | Outgoing::RunSpell(_) => {
            // The framework can replay these back into the reducer; for
            // now we just record the round-trip and leave state alone.
            out.push(o);
            state
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
