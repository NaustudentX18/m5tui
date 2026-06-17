//! Pure application state. The reducer is `Event -> AppState -> AppState`.
//!
//! M1 extends `AppState` with the home-screen fields (mode, focus, agents,
//! session, prompt, palette state, clock, toast) and adds `step`, the
//! side-channel reducer that also returns the `Outgoing` actions the
//! framework must perform.
use crate::event::{Event, Focus, KeyAction, Outgoing};
use m5tui_omp::OmpFrame;
use std::collections::VecDeque;
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
    /// About/version overlay — build info, themes, keybind summary.
    /// Triggered by the `;A` chord (or `;?` from inside the help
    /// overlay).
    About,
    /// Log viewer overlay — scrollable in-memory log buffer with
    /// follow-tail mode. Triggered by the `;L` chord.
    LogViewer,
    /// First-boot boot screen — animated ASCII logo + progress bar,
    /// skippable with any key. The reducer enters `Cockpit` when the
    /// user sends `Outgoing::SkipBoot`.
    Boot,
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

/// A single tool-call entry as ingested from `OmpFrame::ToolCall` and
/// completed by a matching `OmpFrame::ToolResult`. The reducer pushes a
/// fresh card on `ToolCall` and updates `ended` / `output` when the
/// result arrives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmpCard {
    /// Frame id assigned by the OMP server.
    pub id: String,
    /// Tool name (e.g. `read_file`, `shell`).
    pub tool: String,
    /// Ordered (key, value) argument pairs.
    pub args: Vec<(String, String)>,
    /// Tick at which the `ToolCall` arrived.
    pub started: Option<u64>,
    /// Tick at which the matching `ToolResult` arrived.
    pub ended: Option<u64>,
    /// Output from the matching `ToolResult`, if any.
    pub output: Option<String>,
}

/// A todo item extracted from `OmpFrame::TodoUpdate`. Keyed by id;
/// repeated updates with the same id replace the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmpTodo {
    pub id: String,
    pub text: String,
    pub done: bool,
}

/// A subagent entry extracted from `OmpFrame::Subagent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmpSubagent {
    pub id: String,
    pub task: String,
}

/// A profile entry shown in the profile picker. The framework supplies
/// these from the persist layer; the reducer just stores them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSummary {
    pub id: String,
    pub label: String,
    pub host: String,
}

/// A spell entry shown in the book picker. The framework supplies
/// these from `book/*.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellSummary {
    pub id: String,
    pub key: String,
    pub help: String,
}

/// IMU wake behaviour selected on the settings overlay. Cycled by
/// `SettingsLeft` / `SettingsRight` when the cursor sits on the imu-wake
/// row; `Off` means the IMU never wakes the device from sleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImuWake {
    Off,
    #[default]
    Shake,
    Tilt,
}

impl ImuWake {
    /// Next value in the cycle. Wraps from `Tilt` back to `Off`.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Shake,
            Self::Shake => Self::Tilt,
            Self::Tilt => Self::Off,
        }
    }

    /// Previous value in the cycle. Wraps from `Off` back to `Tilt`.
    pub fn prev(self) -> Self {
        match self {
            Self::Off => Self::Tilt,
            Self::Shake => Self::Off,
            Self::Tilt => Self::Shake,
        }
    }

    /// Single-word label for the settings overlay.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Shake => "shake",
            Self::Tilt => "tilt",
        }
    }
}

/// A short-lived message shown in the top-right of the cockpit. Toasts
/// expire when `clock >= until_tick`; the reducer drops them on `Tick`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub text: String,
    pub until_tick: u64,
}

/// In-memory ring buffer of log lines. When the buffer fills, the
/// oldest line is dropped on the next push. The viewer uses
/// `scroll_offset` (0 = follow-tail / newest; 1 = one line up; etc.)
/// and `tail_mode` (auto-reset to bottom on push) to navigate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogBuffer {
    /// Stored lines, oldest-first.
    pub(crate) lines: VecDeque<String>,
    /// Maximum number of lines retained.
    pub(crate) max_lines: usize,
    /// Distance from the newest line. 0 = bottom; 1 = one line older; etc.
    pub(crate) scroll_offset: usize,
    /// When `true`, `push` resets `scroll_offset` to 0 (auto-scroll).
    pub(crate) tail_mode: bool,
}

impl LogBuffer {
    /// Build a buffer that retains up to `max_lines` entries. The new
    /// buffer is empty and starts in follow-tail mode.
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            max_lines: max_lines.max(1),
            scroll_offset: 0,
            tail_mode: true,
        }
    }

    /// Append `line` to the buffer. When the buffer is full, the oldest
    /// line is dropped. If `tail_mode` is `true`, `scroll_offset` is
    /// reset to 0 so the viewer continues to follow the tail.
    pub fn push(&mut self, line: String) {
        if self.lines.len() == self.max_lines {
            self.lines.pop_front();
            // Keep `scroll_offset` honest when the head moves under us.
            if self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        }
        self.lines.push_back(line);
        if self.tail_mode {
            self.scroll_offset = 0;
        }
    }

    /// Iterate the buffered lines from oldest to newest.
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    /// Move one line older (closer to the head). Saturates at the
    /// oldest buffered line. Disables follow-tail when called.
    pub fn scroll_up(&mut self) {
        let max = self.lines.len().saturating_sub(1);
        if self.scroll_offset < max {
            self.scroll_offset += 1;
        }
        self.tail_mode = false;
    }

    /// Move one line newer (closer to the tail). Saturates at 0.
    /// Re-enables follow-tail when the viewer reaches the bottom.
    pub fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
        if self.scroll_offset == 0 {
            self.tail_mode = true;
        }
    }

    /// Jump to the oldest buffered line and disable follow-tail.
    pub fn scroll_top(&mut self) {
        self.scroll_offset = self.lines.len().saturating_sub(1);
        self.tail_mode = false;
    }

    /// Jump back to the newest line and re-enable follow-tail.
    pub fn scroll_bottom(&mut self) {
        self.scroll_offset = 0;
        self.tail_mode = true;
    }

    /// Toggle the follow-tail flag. When toggled off, the current
    /// `scroll_offset` is preserved.
    pub fn toggle_follow(&mut self) {
        self.tail_mode = !self.tail_mode;
        if self.tail_mode {
            self.scroll_offset = 0;
        }
    }

    /// Number of lines currently buffered.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the buffer holds zero lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        // 200 lines ≈ 6 KB of UTF-8 text; fits well in the on-device
        // heap while staying useful for short debugging sessions.
        Self::new(200)
    }
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
    /// When `Some`, the theme editor has a draft theme in flight. The
    /// framework can show "modified" indicators next to menu items and
    /// the `;8`/commit chord targets the draft instead of the active
    /// theme.
    pub theme_draft: Option<m5tui_themes::Theme>,
    /// Current cursor into the theme editor's 9 menu items.
    pub theme_menu_index: usize,
    /// Current draft palette (subset of the full theme) the editor
    /// shows for live preview. Kept as a separate field so the
    /// renderer can read it without cloning the whole theme.
    pub theme_draft_palette: Option<m5tui_themes::ThemePalette>,
    /// In-memory log buffer shown by the `Mode::LogViewer` overlay.
    pub log_buffer: LogBuffer,
    /// In-flight tool calls, oldest-first. Populated from
    /// `OmpFrame::ToolCall` and updated when `ToolResult` arrives.
    pub omp_cards: Vec<OmpCard>,
    /// Todo list keyed by id. Populated from `OmpFrame::TodoUpdate`;
    /// repeated updates with the same id replace the entry.
    pub omp_todos: Vec<OmpTodo>,
    /// Subagents spawned by the OMP session, oldest-first.
    pub omp_subagents: Vec<OmpSubagent>,
    /// Current "thinking" text from the agent, if any. Cleared when
    /// `OmpFrame::Answer` arrives.
    pub omp_thinking: Option<String>,
    /// Recent answers from the agent, newest at the back. Capped at
    /// `OMP_ANSWERS_CAP` (20).
    pub omp_answers: VecDeque<String>,
    /// Profile list shown in `Mode::ProfilePicker`. The framework
    /// supplies these from the persist layer; the reducer stores and
    /// navigates them.
    pub profiles: Vec<ProfileSummary>,
    /// Spell list shown in `Mode::Book`.
    pub book_spells: Vec<SpellSummary>,
    /// Picker cursor used by `Mode::ProfilePicker` and `Mode::Book`.
    /// Both overlays share this field; the reducer clamps it to the
    /// active list length when navigating.
    pub picker_index: usize,
    /// Cursor row in `Mode::Settings`. Indexes into the settings
    /// overlay's row list (see `SETTINGS_ROWS`).
    pub settings_cursor: usize,
    /// LCD backlight brightness (0..=100). Edited in `Mode::Settings`
    /// via `SettingsLeft` / `SettingsRight` on the brightness row.
    pub settings_brightness: u8,
    /// Whether the speaker / click sounds are enabled. Toggled in
    /// `Mode::Settings` on the sound row.
    pub settings_sound: bool,
    /// IMU wake behaviour. Cycled in `Mode::Settings` on the imu-wake
    /// row.
    pub settings_imu_wake: ImuWake,
    /// SSID of the currently-connected Wi-Fi network. Read-only on
    /// the settings overlay; the framework refreshes it from the
    /// device's wifi subsystem.
    pub settings_wifi_ssid: String,
    /// Tailscale status text shown on the settings overlay. Read-only.
    pub settings_tailscale_status: String,
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
            log_buffer: LogBuffer::default(),
            theme_draft: None,
            theme_menu_index: 0,
            theme_draft_palette: None,
            omp_cards: Vec::new(),
            omp_todos: Vec::new(),
            omp_subagents: Vec::new(),
            omp_thinking: None,
            omp_answers: VecDeque::new(),
            profiles: Vec::new(),
            book_spells: Vec::new(),
            picker_index: 0,
            settings_cursor: 0,
            settings_brightness: 77,
            settings_sound: true,
            settings_imu_wake: ImuWake::default(),
            settings_wifi_ssid: "aiserver-5g".into(),
            settings_tailscale_status: "up (100.127.x.x)".into(),
        }
    }
}

/// Maximum number of recent answers retained in `AppState::omp_answers`.
/// The reducer drops the oldest entry when the deque grows past this cap.
pub const OMP_ANSWERS_CAP: usize = 20;

impl AppState {
    /// Construct the state the framework starts the host binary in:
    /// `Mode::Boot` so the boot screen renders before the user sees the
    /// cockpit. The default (`AppState::default()`) is `Mode::Cockpit`,
    /// which is what every reducer unit test wants.
    pub fn booting() -> Self {
        Self {
            mode: Mode::Boot,
            ..Self::default()
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
        // Key, Outgoing, CloseOverlay, and OmpFrameReceived are
        // handled by `step`; the pure projection just bumps the clock
        // so toasts still expire even when the framework skips the
        // side channel.
        Event::Key(_) | Event::Outgoing(_) | Event::CloseOverlay | Event::OmpFrameReceived(_) => {
            AppState {
                clock: state.clock + 1,
                ..state
            }
        }
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
        Event::CloseOverlay => AppState {
            mode: Mode::Cockpit,
            ..state
        },
        Event::Key(action) => apply_key(state, action, &mut out),
        Event::Outgoing(o) => apply_outgoing(state, o, &mut out),
        Event::OmpFrameReceived(frame) => ingest_omp_frame(state, frame),
    };
    (next, out)
}

/// Route an incoming `OmpFrame` into the right `AppState` field. Pure:
/// no I/O, no allocation beyond what the new entry needs.
fn ingest_omp_frame(mut state: AppState, frame: OmpFrame) -> AppState {
    match frame {
        OmpFrame::ToolCall { id, tool, args } => {
            state.omp_cards.push(OmpCard {
                id,
                tool,
                args: args.into_iter().collect(),
                started: Some(state.clock),
                ended: None,
                output: None,
            });
        }
        OmpFrame::ToolResult { id, output } => {
            if let Some(card) = state.omp_cards.iter_mut().find(|c| c.id == id) {
                card.ended = Some(state.clock);
                card.output = Some(output);
            }
        }
        OmpFrame::TodoUpdate { id, text, done } => {
            if let Some(existing) = state.omp_todos.iter_mut().find(|t| t.id == id) {
                existing.text = text;
                existing.done = done;
            } else {
                state.omp_todos.push(OmpTodo { id, text, done });
            }
        }
        OmpFrame::Subagent { id, task } => {
            state.omp_subagents.push(OmpSubagent { id, task });
        }
        OmpFrame::Thinking { text, .. } => {
            state.omp_thinking = Some(text);
        }
        OmpFrame::Answer { text, .. } => {
            state.omp_thinking = None;
            push_answer(&mut state.omp_answers, text);
        }
        // Ask/StreamingChunk/Error don't mutate cockpit state in v1.12.
        OmpFrame::Ask { .. } | OmpFrame::StreamingChunk { .. } | OmpFrame::Error { .. } => {}
    }
    state
}

/// Push `answer` onto the deque, then truncate to `OMP_ANSWERS_CAP`.
/// Centralised so tests can exercise the cap behaviour directly.
fn push_answer(deque: &mut VecDeque<String>, answer: String) {
    deque.push_back(answer);
    while deque.len() > OMP_ANSWERS_CAP {
        deque.pop_front();
    }
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

/// Fork the active theme into a draft. The editor uses this when the
/// user opens the "colors" sub-screen and starts editing swatches.
fn fork_draft_theme(state: AppState) -> AppState {
    if state.theme_draft.is_some() {
        return state;
    }
    // No live theme in AppState yet; the framework will call
    // `Outgoing::RequestTheme` so the device side can supply the
    // current one. Until then we seed a sensible default.
    let draft = m5tui_themes::builtin("coldwire");
    match draft {
        Some(t) => AppState {
            theme_draft: Some(t),
            ..state
        },
        None => state,
    }
}

/// Commit the draft theme to the persist layer. Emits
/// `Outgoing::SaveTheme(draft)` and clears the draft pointer.
fn commit_draft_theme(state: AppState, out: &mut Vec<Outgoing>) -> AppState {
    if let Some(draft) = state.theme_draft.clone() {
        out.push(Outgoing::SaveTheme(Box::new(draft)));
    }
    AppState {
        theme_draft: None,
        ..state
    }
}

/// Discard the draft theme. No `Outgoing` is emitted.
fn discard_draft_theme(state: AppState) -> AppState {
    AppState {
        theme_draft: None,
        ..state
    }
}

fn apply_key(state: AppState, action: KeyAction, out: &mut Vec<Outgoing>) -> AppState {
    // While the boot screen is up, any key skips it. This must run
    // before the per-action match so chords like `;/` or arrow keys
    // also abort the boot rather than being silently absorbed.
    if state.mode == Mode::Boot {
        out.push(Outgoing::SkipBoot);
        return AppState {
            mode: Mode::Cockpit,
            ..state
        };
    }
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
        KeyAction::OpenAbout => open_overlay(state, Mode::About, Outgoing::OpenAbout, out),
        KeyAction::OpenMemory => open_overlay(state, Mode::Memory, Outgoing::OpenMemory, out),
        KeyAction::OpenLogViewer => {
            open_overlay(state, Mode::LogViewer, Outgoing::OpenLogViewer, out)
        }
        KeyAction::LogScrollUp => {
            if state.mode == Mode::LogViewer {
                let mut buf = state.log_buffer;
                buf.scroll_up();
                AppState {
                    log_buffer: buf,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::LogScrollDown => {
            if state.mode == Mode::LogViewer {
                let mut buf = state.log_buffer;
                buf.scroll_down();
                AppState {
                    log_buffer: buf,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::LogScrollTop => {
            if state.mode == Mode::LogViewer {
                let mut buf = state.log_buffer;
                buf.scroll_top();
                AppState {
                    log_buffer: buf,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::LogScrollBottom => {
            if state.mode == Mode::LogViewer {
                let mut buf = state.log_buffer;
                buf.scroll_bottom();
                AppState {
                    log_buffer: buf,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::LogToggleFollow => {
            if state.mode == Mode::LogViewer {
                let mut buf = state.log_buffer;
                buf.toggle_follow();
                AppState {
                    log_buffer: buf,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::SaveMemo => save_memo(state, out),
        KeyAction::ForkDraftTheme => fork_draft_theme(state),
        KeyAction::CommitDraftTheme => commit_draft_theme(state, out),
        KeyAction::DiscardDraftTheme => discard_draft_theme(state),
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
            } else if state.mode == Mode::Settings {
                AppState {
                    settings_cursor: state.settings_cursor.saturating_sub(1),
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
            } else if state.mode == Mode::Settings {
                let max = settings_max_row();
                let next = if state.settings_cursor >= max {
                    max
                } else {
                    state.settings_cursor + 1
                };
                AppState {
                    settings_cursor: next,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::SettingsUp => {
            if state.mode == Mode::Settings {
                AppState {
                    settings_cursor: state.settings_cursor.saturating_sub(1),
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::SettingsDown => {
            if state.mode == Mode::Settings {
                let max = settings_max_row();
                let next = if state.settings_cursor >= max {
                    max
                } else {
                    state.settings_cursor + 1
                };
                AppState {
                    settings_cursor: next,
                    ..state
                }
            } else {
                state
            }
        }
        KeyAction::SettingsLeft => {
            if state.mode == Mode::Settings {
                settings_adjust(state, SettingsDelta::Left)
            } else {
                state
            }
        }
        KeyAction::SettingsRight => {
            if state.mode == Mode::Settings {
                settings_adjust(state, SettingsDelta::Right)
            } else {
                state
            }
        }
        KeyAction::SettingsToggle => {
            if state.mode == Mode::Settings {
                settings_toggle(state)
            } else {
                state
            }
        }
        KeyAction::Enter => {
            if state.mode == Mode::Settings {
                return settings_toggle(state);
            }
            // Picker overlays have their own Enter semantics. Clone
            // the active list before moving `state` into the helper.
            match state.mode {
                Mode::ProfilePicker => {
                    let profiles = state.profiles.clone();
                    return enter_picker(state, &profiles, Outgoing::PickProfile, out);
                }
                Mode::Book => {
                    let spells = state.book_spells.clone();
                    return enter_picker(state, &spells, Outgoing::RunSpell, out);
                }
                _ => {}
            }
            match state.focus {
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
            }
        }
        KeyAction::ProfileUp => {
            let n = state.profiles.len();
            if state.mode == Mode::ProfilePicker {
                picker_move(state, n, PickerDelta::Up)
            } else {
                state
            }
        }
        KeyAction::ProfileDown => {
            let n = state.profiles.len();
            if state.mode == Mode::ProfilePicker {
                picker_move(state, n, PickerDelta::Down)
            } else {
                state
            }
        }
        KeyAction::BookUp => {
            let n = state.book_spells.len();
            if state.mode == Mode::Book {
                picker_move(state, n, PickerDelta::Up)
            } else {
                state
            }
        }
        KeyAction::BookDown => {
            let n = state.book_spells.len();
            if state.mode == Mode::Book {
                picker_move(state, n, PickerDelta::Down)
            } else {
                state
            }
        }
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

/// Direction of a picker cursor move.
enum PickerDelta {
    Up,
    Down,
}

/// Move `state.picker_index` by `delta`, wrapping at the bounds. The
/// empty-list case is a no-op (the cursor stays at 0).
fn picker_move(state: AppState, len: usize, delta: PickerDelta) -> AppState {
    if len == 0 {
        return AppState {
            picker_index: 0,
            ..state
        };
    }
    let next = match delta {
        PickerDelta::Up => {
            if state.picker_index == 0 {
                len - 1
            } else {
                state.picker_index - 1
            }
        }
        PickerDelta::Down => (state.picker_index + 1) % len,
    };
    AppState {
        picker_index: next,
        ..state
    }
}

/// Number of rows shown on the settings overlay. Keep in sync with the
/// ordering in `widgets::overlay::render_settings` and the cursor
/// dispatch in `settings_adjust` / `settings_toggle`.
pub const SETTINGS_ROWS: usize = 5;
/// Brightness step (out of 100) used by `SettingsLeft` / `SettingsRight`
/// on the brightness row.
pub const BRIGHTNESS_STEP: i16 = 5;

fn settings_max_row() -> usize {
    SETTINGS_ROWS - 1
}

/// Direction of a value adjustment on the settings overlay.
enum SettingsDelta {
    Left,
    Right,
}

/// Adjust the value under the settings cursor in `delta`. Brightness
/// rows step by `BRIGHTNESS_STEP`; the imu-wake row cycles through the
/// three `ImuWake` variants. Read-only rows (wifi, tailscale) ignore the
/// keypress.
fn settings_adjust(state: AppState, delta: SettingsDelta) -> AppState {
    match state.settings_cursor {
        0 => {
            let cur = state.settings_brightness as i16;
            let next = match delta {
                SettingsDelta::Left => cur.saturating_sub(BRIGHTNESS_STEP).max(0),
                SettingsDelta::Right => cur.saturating_add(BRIGHTNESS_STEP).min(100),
            };
            AppState {
                settings_brightness: next as u8,
                ..state
            }
        }
        4 => {
            let next = match delta {
                SettingsDelta::Left => state.settings_imu_wake.prev(),
                SettingsDelta::Right => state.settings_imu_wake.next(),
            };
            AppState {
                settings_imu_wake: next,
                ..state
            }
        }
        // Read-only rows: wifi (1), tailscale (2). No-op.
        _ => state,
    }
}

/// Toggle or cycle the value under the settings cursor. Sound row (3)
/// flips the bool; imu-wake row (4) cycles forward; read-only rows
/// ignore.
fn settings_toggle(state: AppState) -> AppState {
    match state.settings_cursor {
        3 => AppState {
            settings_sound: !state.settings_sound,
            ..state
        },
        4 => AppState {
            settings_imu_wake: state.settings_imu_wake.next(),
            ..state
        },
        _ => state,
    }
}

/// Confirm the currently-highlighted picker entry. Emits `mk_out(id)`
/// with the entry's id and keeps the state in the picker mode (the
/// framework will translate the emitted `Outgoing` into a connection
/// or a spell run). When the list is empty the key is a no-op.
fn enter_picker<T, F>(state: AppState, list: &[T], mk_out: F, out: &mut Vec<Outgoing>) -> AppState
where
    F: FnOnce(String) -> Outgoing,
    T: PickerEntry,
{
    if list.is_empty() {
        return state;
    }
    let idx = state.picker_index.min(list.len() - 1);
    let entry = &list[idx];
    let id = entry_id(entry);
    out.push(mk_out(id));
    state
}

/// Extract the id field from a `ProfileSummary` or `SpellSummary`.
/// Both types carry an `id: String` field, so we delegate to the field
/// projection to keep `enter_picker` polymorphic.
fn entry_id<T: PickerEntry>(entry: &T) -> String {
    entry.id().to_string()
}

trait PickerEntry {
    fn id(&self) -> &str;
}
impl PickerEntry for ProfileSummary {
    fn id(&self) -> &str {
        &self.id
    }
}
impl PickerEntry for SpellSummary {
    fn id(&self) -> &str {
        &self.id
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
        Outgoing::OpenAbout => AppState {
            mode: Mode::About,
            ..state
        },
        Outgoing::OpenMemory => AppState {
            mode: Mode::Memory,
            ..state
        },
        Outgoing::OpenLogViewer => AppState {
            mode: Mode::LogViewer,
            ..state
        },
        Outgoing::LogAppend(line) => {
            let mut buf = state.log_buffer;
            buf.push(line);
            AppState {
                log_buffer: buf,
                ..state
            }
        }
        Outgoing::Quit => AppState {
            should_quit: true,
            ..state
        },
        Outgoing::SkipBoot => AppState {
            // Boot screen has ended -- the framework already consumed
            // the keypress; just return to the cockpit.
            mode: Mode::Cockpit,
            ..state
        },
        Outgoing::SubmitPrompt(_)
        | Outgoing::SelectAgent(_)
        | Outgoing::CycleFocus(_)
        | Outgoing::SaveMemo(_)
        | Outgoing::PickProfile(_)
        | Outgoing::RunSpell(_)
        | Outgoing::SaveTheme(_) => {
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
