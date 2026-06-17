//! Events fed into the pure reducer.
//!
//! M0 had only `Tick` and `Quit`. M1 adds the input layer (`Key`) and the
//! side-effect carrier (`Outgoing`) plus the `CloseOverlay` event used by
//! the framework to pop the palette/help modal.

/// Which pane of the cockpit has keyboard focus. Cycled by `Tab`:
/// `Prompt -> Agents -> Session -> Prompt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The bottom-bar prompt line. Char input lands here.
    Prompt,
    /// The left-pane agent list. Up/Down moves the selection.
    Agents,
    /// The right-pane session state. Read-only in M1.
    Session,
}

/// to one of these by `keymap::parse_chord`; the reducer only ever sees
/// `KeyAction` values, never raw scan codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Open the command palette. Also used as the second key of the `;/`
    /// chord (re-enter the palette with the chord prefix consumed).
    Palette,
    /// Open the hotkey/help overlay. Triggered by the `;?` chord.
    Help,
    /// Open the on-device theme editor. Triggered by the `;t` chord.
    OpenThemeEditor,
    /// Open the profile picker. Triggered by the `;p` chord.
    OpenProfilePicker,
    /// Open the book of commands. Triggered by the `;b` chord.
    OpenBook,
    /// Open the voice menu. Triggered by the `;v` chord.
    OpenVoice,
    /// Open the first-boot wizard. Triggered by the `;n` chord.
    OpenFirstBoot,
    /// Open the log viewer overlay. Triggered by the `;L` chord.
    OpenLogViewer,
    /// In the log viewer, scroll one line up (older).
    LogScrollUp,
    /// In the log viewer, scroll one line down (newer).
    LogScrollDown,
    /// In the log viewer, jump to the oldest buffered line.
    LogScrollTop,
    /// In the log viewer, jump back to the newest line and re-enable
    /// follow-tail.
    LogScrollBottom,
    /// In the profile picker, move selection up. Triggered by `k`
    /// while `Mode::ProfilePicker` is active.
    ProfileUp,
    /// In the profile picker, move selection down. Triggered by `j`
    /// while `Mode::ProfilePicker` is active.
    ProfileDown,
    /// In the book/spell picker, move selection up. Triggered by `k`
    /// while `Mode::Book` is active.
    BookUp,
    /// In the book/spell picker, move selection down. Triggered by `j`
    /// while `Mode::Book` is active.
    BookDown,
    /// In the log viewer, toggle the follow-tail flag.
    LogToggleFollow,
    /// Open the device settings screen. Triggered by the `;s` chord.
    OpenSettings,
    /// Run the doctor self-check. Triggered by the `;D` chord.
    RunDoctor,
    /// Open the handoff picker. Triggered by the `;h` chord.
    OpenHandoff,
    /// Open the about/version sheet. Triggered by the `;A` chord
    /// (and also `;?` while the help overlay is already open).
    OpenAbout,
    /// Open the memory/vault search. Triggered by the `;m` chord.
    OpenMemory,
    /// Save a memo. Triggered by the `;w` chord.
    SaveMemo,
    /// Enter the theme editor's edit mode by forking the active theme
    /// into a draft. Triggered by the `;8` chord (or by pressing 8 in
    /// the editor menu).
    ForkDraftTheme,
    /// Commit the current draft theme back to the active theme. Sent
    /// by the editor's "save" item.
    CommitDraftTheme,
    /// Discard the current draft theme.
    DiscardDraftTheme,
    Esc,
    /// Submit the prompt or confirm a selection.
    Enter,
    /// Move selection up in the agent list.
    Up,
    /// Move selection down in the agent list.
    Down,
    Left,
    Right,
    /// Cycle focus: Prompt -> Agents -> Session -> Prompt.
    Tab,
    /// Pop the last char from the prompt.
    Backspace,
    /// Any other printable char. The reducer pushes it into the prompt
    /// when `focus == Focus::Prompt` and ignores it elsewhere.
    Char(char),
}

/// A side-effect the reducer wants the framework to perform. The reducer
/// itself stays pure; `step` is the side-channel variant that returns
/// these along with the next state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// Open the command palette modal.
    OpenPalette,
    /// Open the help overlay.
    OpenHelp,
    /// Open the log viewer overlay.
    OpenLogViewer,
    /// Append a line to the in-memory log buffer. Emitted by the
    /// framework whenever a new log entry arrives.
    LogAppend(String),
    /// Close whichever modal is open and return to the cockpit.
    CloseOverlay,
    /// The user pressed Enter in the prompt with a non-empty message.
    /// The framework will route this to OMP (or the local echo in M1).
    SubmitPrompt(String),
    /// The user picked `quit` from the palette or pressed a quit chord.
    Quit,
    /// Skip the boot screen and jump to the cockpit. Emitted by the
    /// boot widget when the user presses any key while `Mode::Boot`
    /// is active.
    SkipBoot,
    /// The user selected the agent at `index` in the agent list.
    SelectAgent(usize),
    /// The user pressed Tab; the framework may want to play a focus
    /// change sound or update a status line.
    CycleFocus(Focus),
    /// Open the profile picker overlay.
    OpenProfilePicker,
    /// Open the book of commands overlay.
    OpenBook,
    /// Open the voice menu overlay.
    OpenVoice,
    /// Open the first-boot wizard overlay.
    OpenFirstBoot,
    /// Open the device settings overlay.
    OpenSettings,
    /// Run the doctor self-check and render the scoreboard.
    RunDoctor,
    /// Open the handoff picker overlay.
    OpenHandoff,
    /// Open the about/version overlay.
    OpenAbout,
    /// Open the memory/vault search overlay.
    OpenMemory,
    /// Save a memo note; the framework should append to /sd/m5tui/memos/&lt;date&gt;.md.
    SaveMemo(String),
    /// Run a profile pick result — `id` is the selected profile identifier.
    PickProfile(String),
    /// Run a book spell — `id` is the selected spell identifier.
    RunSpell(String),
    /// Save the current draft theme. The framework writes it to
    /// `/sd/m5tui/themes/<name>.yaml` via the persist `Driver`.
    SaveTheme(Box<m5tui_themes::Theme>),
}

/// All events the reducer accepts. `Tick` and `Quit` were M0; everything
/// else is M1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Tick,
    Quit,
    Key(KeyAction),
    Outgoing(Outgoing),
    /// The framework telling the reducer to drop whichever modal is open.
    /// This is a duplicate of `Outgoing(Outgoing::CloseOverlay)` but it
    /// keeps the public surface symmetric (one variant per direction).
    CloseOverlay,
    /// A frame arrived from the OMP transport. The framework injects
    /// this whenever `m5tui_omp::OmpSession::poll` returns a frame.
    /// The reducer routes it into the right `AppState` field.
    OmpFrameReceived(m5tui_omp::OmpFrame),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_and_quit_are_distinct() {
        assert_ne!(Event::Tick, Event::Quit);
    }

    #[test]
    fn key_and_outgoing_are_distinct() {
        assert_ne!(
            Event::Key(KeyAction::Enter),
            Event::Outgoing(Outgoing::CloseOverlay),
        );
    }

    #[test]
    fn close_overlay_event_is_a_variant() {
        let e = Event::CloseOverlay;
        assert_eq!(e, Event::CloseOverlay);
    }

    #[test]
    fn keyaction_char_carries_payload() {
        let k = KeyAction::Char('h');
        match k {
            KeyAction::Char(c) => assert_eq!(c, 'h'),
            _ => panic!("expected Char variant"),
        }
    }
}
